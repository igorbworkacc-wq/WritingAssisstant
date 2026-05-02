use crate::errors::{AppError, AppResult};
use crate::model_settings::{self, ModelSettings};
use crate::secure_store;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tauri::AppHandle;

const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const OPENAI_MODELS_URL: &str = "https://api.openai.com/v1/models";

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

#[derive(Debug, Clone, Default)]
struct ResponseOptions {
    temperature: Option<f32>,
    max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
struct OpenAiSafeError {
    user_message: String,
    http_status: Option<u16>,
    error_type: Option<String>,
    error_code: Option<String>,
}

pub fn build_prompt(original_text: &str, transform_type: TransformType) -> PromptParts {
    match transform_type {
        TransformType::Correction => PromptParts {
            system: "You are a strict grammar correction engine for corporate text. Correct only objective grammar, spelling, punctuation, and syntax errors. Preserve the writer's tone, formality, intent, sentence structure, and vocabulary unless a change is necessary for correctness. Return only the corrected text.",
            user: format!(
                "Correct the following text. Return only the corrected text.\n\n<text>\n{}\n</text>",
                original_text
            ),
        },
        TransformType::Rephrase => PromptParts {
            system: "You are a professional corporate writing assistant. Rewrite text into clear, professional, B2-level English while preserving the original meaning, facts, and intent. Improve clarity, flow, and professionalism. Do not add facts. Do not remove important information. Return only the rewritten text.",
            user: format!(
                "Rephrase the following text into professional B2-level English. Return only the rewritten text.\n\n<text>\n{}\n</text>",
                original_text
            ),
        },
    }
}

pub async fn call_openai_correction(app: AppHandle, original_text: String) -> AppResult<String> {
    call_openai_text_transform(&app, original_text, TransformType::Correction).await
}

pub async fn call_openai_rephrase(app: AppHandle, original_text: String) -> AppResult<String> {
    call_openai_text_transform(&app, original_text, TransformType::Rephrase).await
}

pub async fn test_api_key() -> AppResult<()> {
    let api_key = secure_store::get_api_key()?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| AppError::Network)?;

    let response = client
        .get(OPENAI_MODELS_URL)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|_| AppError::Network)?;

    map_api_key_status(response.status())
}

pub async fn test_selected_model(app: AppHandle) -> AppResult<()> {
    let settings = model_settings::get_model_settings(&app)?;
    test_model_with_settings(settings).await
}

pub async fn test_model(model: String) -> AppResult<()> {
    test_model_with_settings(ModelSettings {
        selected_model: model,
        temperature: 1.0,
    })
    .await
}

pub async fn list_available_models() -> AppResult<Vec<String>> {
    let api_key = secure_store::get_api_key()?;
    list_available_models_with_key(&api_key).await
}

pub async fn is_model_available(model: String) -> AppResult<bool> {
    let model = model.trim();
    if model.is_empty() {
        return Ok(false);
    }

    let models = list_available_models().await?;
    Ok(models.iter().any(|available| available == model))
}

async fn test_model_with_settings(settings: ModelSettings) -> AppResult<()> {
    let api_key = secure_store::get_api_key()?;
    let selected_model = settings.selected_model.trim();
    if selected_model.is_empty() {
        return Err(AppError::ModelNotAvailableForKey);
    }

    let available_models = list_available_models_with_key(&api_key).await?;
    if !available_models
        .iter()
        .any(|available| available == selected_model)
    {
        return Err(AppError::ModelNotAvailableForKey);
    }

    create_response_text(
        &api_key,
        selected_model,
        "You are a minimal API test responder.",
        "Return only the word OK.",
        ResponseOptions::default(),
    )
    .await
    .map(|_| ())
}

async fn list_available_models_with_key(api_key: &str) -> AppResult<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| AppError::Network)?;

    let response = client
        .get(OPENAI_MODELS_URL)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|_| AppError::Network)?;

    let status = response.status();
    if !status.is_success() {
        map_api_key_status(status)?;
        return Err(AppError::Network);
    }

    let parsed: OpenAiModelsResponse = response
        .json()
        .await
        .map_err(|_| AppError::UnexpectedResponseFormat)?;
    Ok(parsed.data.into_iter().map(|model| model.id).collect())
}

async fn call_openai_text_transform(
    app: &AppHandle,
    original_text: String,
    transform_type: TransformType,
) -> AppResult<String> {
    let api_key = secure_store::get_api_key()?;
    let model_settings = model_settings::get_model_settings(app)?;
    let prompt = build_prompt(&original_text, transform_type);
    create_response_text(
        &api_key,
        model_settings.selected_model.trim(),
        prompt.system,
        &prompt.user,
        ResponseOptions::default(),
    )
    .await
}

async fn create_response_text(
    api_key: &str,
    model: &str,
    instructions: &str,
    input: &str,
    options: ResponseOptions,
) -> AppResult<String> {
    if model.trim().is_empty() {
        return Err(AppError::ModelNotAvailableForKey);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| AppError::Network)?;

    let body = build_response_body(model, instructions, input, &options);
    let response = client
        .post(OPENAI_RESPONSES_URL)
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
    if status.is_success() {
        let parsed: OpenAiResponse = response
            .json()
            .await
            .map_err(|_| AppError::UnexpectedResponseFormat)?;
        return parsed
            .output_text()
            .ok_or(AppError::UnexpectedResponseFormat);
    }

    let body = response.text().await.unwrap_or_default();
    classify_openai_error(status, &body)
}

fn build_response_body(
    model: &str,
    instructions: &str,
    input: &str,
    options: &ResponseOptions,
) -> Value {
    let mut body = json!({
        "model": model.trim(),
        "instructions": instructions,
        "input": input
    });

    if let Some(temperature) = options.temperature {
        body["temperature"] = json!(temperature);
    }
    if let Some(max_output_tokens) = options.max_output_tokens {
        body["max_output_tokens"] = json!(max_output_tokens);
    }

    body
}

fn classify_openai_error(status: StatusCode, body: &str) -> AppResult<String> {
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(AppError::Authentication);
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(AppError::RateLimited);
    }

    let safe_error = parse_openai_safe_error(status, body);
    let _ = (
        &safe_error.user_message,
        safe_error.http_status,
        &safe_error.error_type,
        &safe_error.error_code,
    );

    if is_optional_parameter_error(&safe_error) {
        return Err(AppError::UnsupportedParameter);
    }
    if is_model_unavailable_error(&safe_error) {
        return Err(AppError::ModelNotAvailableForKey);
    }
    if status == StatusCode::BAD_REQUEST
        || status == StatusCode::NOT_FOUND
        || status.as_u16() == 422
    {
        return Err(AppError::RequestFormat);
    }
    if !status.is_success() {
        return Err(AppError::Network);
    }

    Err(AppError::Network)
}

fn map_api_key_status(status: StatusCode) -> AppResult<()> {
    if status.is_success() {
        return Ok(());
    }
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(AppError::Authentication);
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(AppError::RateLimited);
    }

    Err(AppError::Network)
}

fn parse_openai_safe_error(status: StatusCode, body: &str) -> OpenAiSafeError {
    let parsed: Option<OpenAiErrorEnvelope> = serde_json::from_str(body).ok();
    let error = parsed.and_then(|envelope| envelope.error);

    OpenAiSafeError {
        user_message: if status.is_success() {
            String::new()
        } else {
            AppError::RequestFormat.user_message().to_string()
        },
        http_status: Some(status.as_u16()),
        error_type: error.as_ref().and_then(|value| value.error_type.clone()),
        error_code: error.and_then(|value| match value.code {
            Some(OpenAiErrorCode::String(code)) => Some(code),
            Some(OpenAiErrorCode::Number(code)) => Some(code.to_string()),
            None => None,
        }),
    }
}

fn is_optional_parameter_error(error: &OpenAiSafeError) -> bool {
    error
        .error_code
        .as_deref()
        .map(|code| {
            let lower = code.to_ascii_lowercase();
            lower.contains("unsupported_parameter")
                || lower.contains("unknown_parameter")
                || lower.contains("invalid_parameter")
        })
        .unwrap_or(false)
}

fn is_model_unavailable_error(error: &OpenAiSafeError) -> bool {
    let error_type = error
        .error_type
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let error_code = error
        .error_code
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    error_code.contains("model_not_found")
        || error_code.contains("model_not_available")
        || error_code.contains("model_access")
        || error_code.contains("unsupported_model")
        || error_type.contains("model_not_found")
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiModel {
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiErrorEnvelope {
    error: Option<OpenAiErrorPayload>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiErrorPayload {
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: Option<OpenAiErrorCode>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum OpenAiErrorCode {
    String(String),
    Number(i64),
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
    use super::{build_prompt, TransformType};
    use crate::model_settings::ModelSettings;

    #[test]
    fn correction_prompt_preserves_strict_scope() {
        let prompt = build_prompt("I has a pen.", TransformType::Correction);
        assert!(prompt.system.contains("Correct only objective grammar"));
        assert!(prompt.user.contains("<text>\nI has a pen.\n</text>"));
    }

    #[test]
    fn rephrase_prompt_targets_b2_corporate_english() {
        let prompt = build_prompt("pls review", TransformType::Rephrase);
        assert!(prompt.system.contains("B2-level English"));
        assert!(prompt.user.contains("Return only the rewritten text."));
    }

    #[test]
    fn default_model_and_temperature_match_product_requirement() {
        let settings = ModelSettings::default();
        assert_eq!(settings.selected_model, "gpt-5-nano");
        assert_eq!(settings.temperature, 1.0);
    }
}
