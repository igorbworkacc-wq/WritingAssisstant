use serde::ser::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("No text was selected or copied.")]
    EmptySelection,
    #[error("OpenAI API key is missing.")]
    MissingApiKey,
    #[error("OpenAI authentication failed. Please replace your API key.")]
    Authentication,
    #[error("OpenAI rate limit reached. Please try again later.")]
    RateLimited,
    #[error("Network request failed. Check your connection.")]
    Network,
    #[error("OpenAI request timed out. Please try again.")]
    Timeout,
    #[error("The original application window is no longer available.")]
    TargetWindowUnavailable,
    #[error("Clipboard access failed. Another application may be locking the clipboard.")]
    ClipboardUnavailable,
    #[error("Shortcut registration failed. Ctrl+Space may already be in use.")]
    ShortcutRegistration,
    #[error("A writing operation is already in progress.")]
    OperationAlreadyActive,
    #[error("The requested writing operation is no longer active.")]
    OperationNotFound,
    #[error("OpenAI returned an empty response.")]
    EmptyApiResponse,
    #[error("The API key could not be saved securely.")]
    SecureStore,
    #[error("The selected OpenAI model is unavailable or invalid. Please choose another model in Settings.")]
    ModelUnavailable,
    #[error("The selected model is not available for this API key. Choose another model from the available models list.")]
    ModelNotAvailableForKey,
    #[error("OpenAI request format failed. Please update the app.")]
    RequestFormat,
    #[error("OpenAI returned an unexpected response format. Please update the app.")]
    UnexpectedResponseFormat,
    #[error("Selected model rejected an optional request parameter.")]
    UnsupportedParameter,
    #[error("Model settings could not be saved or loaded.")]
    Settings,
    #[error("The application window could not be shown.")]
    Window,
}

impl AppError {
    pub fn user_message(&self) -> &'static str {
        match self {
            AppError::EmptySelection => "No text was selected or copied.",
            AppError::MissingApiKey => "OpenAI API key is missing.",
            AppError::Authentication => {
                "OpenAI authentication failed. Please replace your API key."
            }
            AppError::RateLimited => "OpenAI rate limit reached. Please try again later.",
            AppError::Network => "Network request failed. Check your connection.",
            AppError::Timeout => "OpenAI request timed out. Please try again.",
            AppError::TargetWindowUnavailable => {
                "The original application window is no longer available."
            }
            AppError::ClipboardUnavailable => {
                "Clipboard access failed. Another application may be locking the clipboard."
            }
            AppError::ShortcutRegistration => {
                "Shortcut registration failed. Ctrl+Space may already be in use."
            }
            AppError::OperationAlreadyActive => "A writing operation is already in progress.",
            AppError::OperationNotFound => "The requested writing operation is no longer active.",
            AppError::EmptyApiResponse => "OpenAI returned an empty response.",
            AppError::SecureStore => "The API key could not be saved securely.",
            AppError::ModelUnavailable => {
                "The selected OpenAI model is unavailable or invalid. Please choose another model in Settings."
            }
            AppError::ModelNotAvailableForKey => {
                "The selected model is not available for this API key. Choose another model from the available models list."
            }
            AppError::RequestFormat => "OpenAI request format failed. Please update the app.",
            AppError::UnexpectedResponseFormat => {
                "OpenAI returned an unexpected response format. Please update the app."
            }
            AppError::UnsupportedParameter => {
                "Selected model rejected an optional request parameter."
            }
            AppError::Settings => "Model settings could not be saved or loaded.",
            AppError::Window => "The application window could not be shown.",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.user_message())
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn safe_error_formatting_does_not_include_internal_details() {
        assert_eq!(
            AppError::Authentication.user_message(),
            "OpenAI authentication failed. Please replace your API key."
        );
        assert!(!AppError::ClipboardUnavailable
            .user_message()
            .contains("clipboard contents"));
    }
}
