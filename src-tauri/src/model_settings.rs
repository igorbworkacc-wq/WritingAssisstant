use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const SETTINGS_FILE: &str = "model-settings.json";
const DEFAULT_MODEL: &str = "gpt-5-nano";
const DEFAULT_TEMPERATURE: f32 = 1.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    #[serde(alias = "selectedModel")]
    pub selected_model: String,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelPreset {
    pub id: String,
    pub label: String,
    pub description: String,
    pub recommended_for: String,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            selected_model: DEFAULT_MODEL.to_string(),
            temperature: DEFAULT_TEMPERATURE,
        }
    }
}

pub fn get_model_presets() -> Vec<ModelPreset> {
    vec![
        ModelPreset {
            id: "gpt-5-nano".to_string(),
            label: "GPT-5 nano".to_string(),
            description: "Lowest-cost GPT-5 class option intended for short text correction and professional rephrasing."
                .to_string(),
            recommended_for: "Recommended default for this app.".to_string(),
        },
        ModelPreset {
            id: "gpt-5-mini".to_string(),
            label: "GPT-5 mini".to_string(),
            description: "More capable than nano while still cost-conscious.".to_string(),
            recommended_for: "Use if nano quality is insufficient.".to_string(),
        },
        ModelPreset {
            id: "gpt-4o-mini".to_string(),
            label: "GPT-4o mini".to_string(),
            description: "Legacy low-cost fallback model.".to_string(),
            recommended_for:
                "Use if GPT-5 nano or GPT-5 mini are unavailable in the API account.".to_string(),
        },
        ModelPreset {
            id: "gpt-5".to_string(),
            label: "GPT-5".to_string(),
            description: "Higher-capability GPT-5 model.".to_string(),
            recommended_for: "Use when output quality is more important than cost.".to_string(),
        },
        ModelPreset {
            id: "custom".to_string(),
            label: "Custom model ID".to_string(),
            description: "Allows entering any OpenAI model ID available to the user's API account."
                .to_string(),
            recommended_for: "Advanced users.".to_string(),
        },
    ]
}

pub fn get_model_settings(app: &AppHandle) -> AppResult<ModelSettings> {
    if let Some(settings) = read_persisted_settings(app)? {
        return Ok(settings);
    }

    if let Ok(model) = std::env::var("OPENAI_MODEL") {
        let selected_model = model.trim();
        if !selected_model.is_empty() {
            return Ok(ModelSettings {
                selected_model: selected_model.to_string(),
                ..ModelSettings::default()
            });
        }
    }

    Ok(ModelSettings::default())
}

pub fn set_model_settings(app: &AppHandle, settings: ModelSettings) -> AppResult<()> {
    let normalized = normalize_settings(settings)?;
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| AppError::Settings)?;
    }

    let json = serde_json::to_string_pretty(&normalized).map_err(|_| AppError::Settings)?;
    fs::write(path, json).map_err(|_| AppError::Settings)?;

    let confirmed = get_model_settings(app)?;
    if confirmed.selected_model == normalized.selected_model
        && (confirmed.temperature - normalized.temperature).abs() < f32::EPSILON
    {
        Ok(())
    } else {
        Err(AppError::Settings)
    }
}

fn read_persisted_settings(app: &AppHandle) -> AppResult<Option<ModelSettings>> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(None);
    }

    let value = fs::read_to_string(path).map_err(|_| AppError::Settings)?;
    let parsed: ModelSettings = serde_json::from_str(&value).map_err(|_| AppError::Settings)?;
    normalize_settings(parsed).map(Some)
}

fn normalize_settings(settings: ModelSettings) -> AppResult<ModelSettings> {
    let selected_model = settings.selected_model.trim().to_string();
    if selected_model.is_empty() || !(0.0..=2.0).contains(&settings.temperature) {
        return Err(AppError::Settings);
    }

    Ok(ModelSettings {
        selected_model,
        temperature: settings.temperature,
    })
}

fn settings_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|_| AppError::Settings)?
        .join(SETTINGS_FILE))
}

#[cfg(test)]
mod tests {
    use super::{ModelSettings, DEFAULT_MODEL, DEFAULT_TEMPERATURE};

    #[test]
    fn default_model_settings_match_product_default() {
        let settings = ModelSettings::default();
        assert_eq!(settings.selected_model, DEFAULT_MODEL);
        assert_eq!(settings.temperature, DEFAULT_TEMPERATURE);
    }
}
