use crate::errors::{AppError, AppResult};
use keyring::Error as KeyringError;
use serde::Serialize;

const KEYRING_SERVICE: &str = "PrivacyTextAssistant";
const KEYRING_USERNAME: &str = "openai_api_key";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyStatus {
    pub env_key_present: bool,
    pub keyring_key_present: bool,
    pub usable_key_present: bool,
}

fn entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USERNAME).map_err(|_| AppError::SecureStore)
}

pub fn get_api_key() -> AppResult<String> {
    if let Ok(value) = std::env::var("OPENAI_API_KEY") {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    let stored = read_keyring_api_key()?;

    if stored.is_empty() {
        Err(AppError::MissingApiKey)
    } else {
        Ok(stored)
    }
}

pub fn has_api_key() -> AppResult<bool> {
    Ok(get_api_key().is_ok())
}

pub fn set_api_key(api_key: String) -> AppResult<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(AppError::MissingApiKey);
    }

    entry()?
        .set_password(trimmed)
        .map_err(|_| AppError::SecureStore)?;

    let retrieved = read_keyring_api_key()?;
    if retrieved != trimmed {
        Err(AppError::SecureStore)
    } else if get_api_key()?.is_empty() {
        Err(AppError::MissingApiKey)
    } else {
        Ok(())
    }
}

pub fn get_api_key_status() -> AppResult<ApiKeyStatus> {
    let env_key_present = std::env::var("OPENAI_API_KEY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let keyring_key_present = read_keyring_api_key()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let usable_key_present = get_api_key().is_ok();

    Ok(ApiKeyStatus {
        env_key_present,
        keyring_key_present,
        usable_key_present,
    })
}

fn read_keyring_api_key() -> AppResult<String> {
    entry()?
        .get_password()
        .map_err(|error| match error {
            KeyringError::NoEntry => AppError::MissingApiKey,
            _ => AppError::SecureStore,
        })
        .map(|value| value.trim().to_string())
}
