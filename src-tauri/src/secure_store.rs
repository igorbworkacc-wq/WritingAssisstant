use crate::errors::{AppError, AppResult};
use keyring::Error as KeyringError;

const SERVICE_NAME: &str = "PrivacyTextAssistant";
const ACCOUNT_NAME: &str = "openai_api_key";

fn entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME).map_err(|_| AppError::SecureStore)
}

pub fn get_api_key() -> AppResult<String> {
    if let Ok(value) = std::env::var("OPENAI_API_KEY") {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    let stored = entry()?
        .get_password()
        .map_err(|error| match error {
            KeyringError::NoEntry => AppError::MissingApiKey,
            _ => AppError::SecureStore,
        })?
        .trim()
        .to_string();

    if stored.is_empty() {
        Err(AppError::MissingApiKey)
    } else {
        Ok(stored)
    }
}

pub fn has_api_key() -> AppResult<bool> {
    match get_api_key() {
        Ok(_) => Ok(true),
        Err(AppError::MissingApiKey) => Ok(false),
        Err(error) => Err(error),
    }
}

pub fn set_api_key(api_key: String) -> AppResult<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(AppError::MissingApiKey);
    }

    entry()?
        .set_password(trimmed)
        .map_err(|_| AppError::SecureStore)?;

    let retrieved = get_api_key()?;
    if retrieved.is_empty() {
        Err(AppError::MissingApiKey)
    } else {
        Ok(())
    }
}
