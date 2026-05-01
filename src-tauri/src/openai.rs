use crate::errors::{AppError, AppResult};
use crate::secure_store;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const MODEL: &str = "gpt-4o-mini";
const TEMPERATURE: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformType {
    Correction,
    Rephrase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptParts {
    pub system: &'static str,
    pub user: String,
}

pub fn build_prompt(original_text: &str, transform_type: TransformType) -> PromptParts {
    match transform_type {
        TransformType::Correction => PromptParts {
            system: "You are a strict grammar correction engine for corporate text. Your task is to correct only objective grammar, spelling, punctuation, and syntax errors. Preserve the writer's tone, formality, intent, sentence structure, and vocabulary unless a change is necessary for correctness. Do not explain anything. Return only the corrected text.",
            user: format!(
                "Correct the following text. Return only the corrected text.\n\n<text>\n{}\n</text>",
                original_text
            ),
        },
        TransformType::Rephrase => PromptParts {
            system: "You are a professional corporate writing assistant. Rewrite text into clear, professional, B2-level English while preserving the original meaning, facts, and intent. Improve clarity, flow, and professionalism. Do not add facts. Do not remove important information. Do not explain anything. Return only the rewritten text.",
            user: format!(
                "Rephrase the following text into professional B2-level English. Return only the rewritten text.\n\n<text>\n{}\n</text>",
                original_text
            ),
        },
    }
}

pub async fn call_openai_correction(original_text: String) -> AppResult<String> {
    call_openai_text_transform(original_text, TransformType::Correction).await
}

pub async fn call_openai_rephrase(original_text: String) -> AppResult<String> {
    call_openai_text_transform(original_text, TransformType::Rephrase).await
}

async fn call_openai_text_transform(
    original_text: String,
    transform_type: TransformType,
) -> AppResult<String> {
    let api_key = secure_store::get_api_key()?;
    let prompt = build_prompt(&original_text, transform_type);
    call_openai_text_transform_with_client(
        &reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| AppError::Network)?,
        OPENAI_RESPONSES_URL,
        &api_key,
        prompt,
    )
    .await
}

async fn call_openai_text_transform_with_client(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    prompt: PromptParts,
) -> AppResult<String> {
    let body = json!({
        "model": MODEL,
        "temperature": TEMPERATURE,
        "instructions": prompt.system,
        "input": prompt.user
    });

    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| {
            if err.is_timeout() {
                AppError::Timeout
            } else {
                AppError::Network
            }
        })?;

    let status = response.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(AppError::Authentication);
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(AppError::RateLimited);
    }
    if !status.is_success() {
        return Err(AppError::Network);
    }

    let parsed: OpenAiResponse = response.json().await.map_err(|_| AppError::Network)?;
    parsed.output_text().ok_or(AppError::EmptyApiResponse)
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiResponse {
    #[serde(default)]
    output_text: Option<String>,
    #[serde(default)]
    output: Vec<ResponseOutputItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseOutputItem {
    #[serde(default)]
    content: Vec<ResponseContentItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseContentItem {
    #[serde(default)]
    text: Option<String>,
}

impl OpenAiResponse {
    fn output_text(self) -> Option<String> {
        if let Some(text) = self.output_text {
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }

        let text = self
            .output
            .into_iter()
            .flat_map(|item| item.content.into_iter())
            .filter_map(|content| content.text)
            .collect::<Vec<_>>()
            .join("");

        let trimmed = text.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::{build_prompt, TransformType, MODEL, TEMPERATURE};

    #[test]
    fn correction_prompt_preserves_strict_scope() {
        let prompt = build_prompt("I has a pen.", TransformType::Correction);
        assert!(prompt.system.contains("correct only objective grammar"));
        assert!(prompt.user.contains("<text>\nI has a pen.\n</text>"));
    }

    #[test]
    fn rephrase_prompt_targets_b2_corporate_english() {
        let prompt = build_prompt("pls review", TransformType::Rephrase);
        assert!(prompt.system.contains("B2-level English"));
        assert!(prompt.user.contains("Return only the rewritten text."));
    }

    #[test]
    fn model_and_temperature_match_product_requirement() {
        assert_eq!(MODEL, "gpt-4o-mini");
        assert_eq!(TEMPERATURE, 1.0);
    }
}
