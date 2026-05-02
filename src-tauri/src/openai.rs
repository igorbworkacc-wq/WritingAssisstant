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

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| AppError::Network)?;

    let body = build_test_body(&settings, true);
    let response = send_openai_request(&client, OPENAI_RESPONSES_URL, &api_key, &body).await?;
    handle_empty_response_or_retry_without_temperature(
        &client,
        OPENAI_RESPONSES_URL,
        &api_key,
        response,
        || build_test_body(&settings, false),
    )
    .await
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
    call_openai_text_transform_with_client(
        &reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| AppError::Network)?,
        OPENAI_RESPONSES_URL,
        &api_key,
        &model_settings,
        prompt,
    )
    .await
}

async fn call_openai_text_transform_with_client(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    model_settings: &ModelSettings,
    prompt: PromptParts,
) -> AppResult<String> {
    let body = build_transform_body(model_settings, &prompt, true);
    let response = send_openai_request(client, url, api_key, &body).await?;
    handle_text_response_or_retry_without_temperature(client, url, api_key, response, || {
        build_transform_body(model_settings, &prompt, false)
    })
    .await
}

async fn send_openai_request(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
) -> AppResult<reqwest::Response> {
    client
        .post(url)
        .bearer_auth(api_key)
        .json(body)
        .send()
        .await
        .map_err(|err| {
            if err.is_timeout() {
                AppError::Timeout
            } else {
                AppError::Network
            }
        })
}

async fn handle_text_response_or_retry_without_temperature<F>(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    response: reqwest::Response,
    retry_body: F,
) -> AppResult<String>
where
    F: FnOnce() -> Value,
{
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
    if status == StatusCode::BAD_REQUEST && is_temperature_unsupported_error(&body) {
        let retry = send_openai_request(client, url, api_key, &retry_body()).await?;
        let retry_status = retry.status();
        if retry_status.is_success() {
            let parsed: OpenAiResponse = retry
                .json()
                .await
                .map_err(|_| AppError::UnexpectedResponseFormat)?;
            return parsed
                .output_text()
                .ok_or(AppError::UnexpectedResponseFormat);
        }
        let retry_body = retry.text().await.unwrap_or_default();
        map_status(retry_status, &retry_body)?;
        return Err(AppError::Network);
    }

    map_status(status, &body)?;
    Err(AppError::Network)
}

async fn handle_empty_response_or_retry_without_temperature<F>(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    response: reqwest::Response,
    retry_body: F,
) -> AppResult<()>
where
    F: FnOnce() -> Value,
{
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().await.unwrap_or_default();
    if status == StatusCode::BAD_REQUEST && is_temperature_unsupported_error(&body) {
        let retry = send_openai_request(client, url, api_key, &retry_body()).await?;
        let retry_status = retry.status();
        if retry_status.is_success() {
            return Ok(());
        }
        let retry_body = retry.text().await.unwrap_or_default();
        return map_status(retry_status, &retry_body);
    }

    map_status(status, &body)
}

fn build_transform_body(
    model_settings: &ModelSettings,
    prompt: &PromptParts,
    include_temperature: bool,
) -> Value {
    let mut body = json!({
        "model": model_settings.selected_model.as_str(),
        "instructions": prompt.system,
        "input": prompt.user,
        "max_output_tokens": 1000
    });

    if include_temperature {
        body["temperature"] = json!(model_settings.temperature);
    }

    body
}

fn build_test_body(model_settings: &ModelSettings, include_temperature: bool) -> Value {
    let mut body = json!({
        "model": model_settings.selected_model.trim(),
        "input": "Return only the word OK.",
        "max_output_tokens": 10
    });

    if include_temperature {
        body["temperature"] = json!(model_settings.temperature);
    }

    body
}

fn map_status(status: StatusCode, body: &str) -> AppResult<()> {
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(AppError::Authentication);
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(AppError::RateLimited);
    }
    if is_model_unavailable_error(body) {
        return Err(AppError::ModelUnavailable);
    }
    if status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND {
        return Err(AppError::RequestFormat);
    }
    if !status.is_success() {
        return Err(AppError::Network);
    }

    Ok(())
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

fn is_temperature_unsupported_error(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("temperature")
        && (lower.contains("unsupported")
            || lower.contains("not supported")
            || lower.contains("unknown parameter")
            || lower.contains("unrecognized")
            || lower.contains("invalid parameter"))
}

fn is_model_unavailable_error(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("invalid model")
        || lower.contains("model")
            && (lower.contains("not found")
                || lower.contains("does not exist")
                || lower.contains("unavailable")
                || lower.contains("not available")
                || lower.contains("no access")
                || lower.contains("not have access")
                || lower.contains("do not have access")
                || lower.contains("unsupported model"))
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
    fn default_model_and_temperature_match_product_requirement() {
        let settings = ModelSettings::default();
        assert_eq!(settings.selected_model, "gpt-5-nano");
        assert_eq!(settings.temperature, 1.0);
    }
}
