use crate::clipboard::{self, ClipboardSnapshot};
use crate::errors::{AppError, AppResult};
use crate::keyboard;
use crate::model_settings::{self, ModelPreset, ModelSettings};
use crate::openai;
use crate::secure_store;
use crate::window_state::{self, CapturedTargetWindow};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

#[derive(Default)]
pub struct AppState {
    pub inner: Arc<Mutex<OperationManager>>,
}

#[derive(Default)]
pub struct OperationManager {
    pub active_operation_id: Option<String>,
    pub operations: HashMap<String, OperationContext>,
}

#[derive(Clone, Debug)]
pub struct OperationContext {
    pub target: CapturedTargetWindow,
    pub clipboard_snapshot: ClipboardSnapshot,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationStartedPayload {
    operation_id: String,
    original_text: String,
    target_captured: bool,
    model: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationErrorPayload {
    message: String,
}

#[tauri::command]
pub fn has_api_key() -> AppResult<bool> {
    secure_store::has_api_key()
}

#[tauri::command]
pub fn set_api_key(api_key: String) -> AppResult<()> {
    secure_store::set_api_key(api_key)
}

#[tauri::command]
pub async fn test_api_key() -> AppResult<()> {
    openai::test_api_key().await
}

#[tauri::command]
pub fn get_model_settings(app: AppHandle) -> AppResult<ModelSettings> {
    model_settings::get_model_settings(&app)
}

#[tauri::command]
pub fn set_model_settings(app: AppHandle, settings: ModelSettings) -> AppResult<()> {
    model_settings::set_model_settings(&app, settings)
}

#[tauri::command]
pub fn get_model_presets() -> AppResult<Vec<ModelPreset>> {
    Ok(model_settings::get_model_presets())
}

#[tauri::command]
pub async fn test_selected_model(app: AppHandle) -> AppResult<()> {
    openai::test_selected_model(app).await
}

#[tauri::command]
pub async fn test_model(model: String) -> AppResult<()> {
    openai::test_model(model).await
}

#[tauri::command]
pub async fn run_correction(
    app: AppHandle,
    _operation_id: String,
    original_text: String,
) -> AppResult<String> {
    openai::call_openai_correction(app, original_text).await
}

#[tauri::command]
pub async fn run_rephrase(
    app: AppHandle,
    _operation_id: String,
    original_text: String,
) -> AppResult<String> {
    openai::call_openai_rephrase(app, original_text).await
}

#[tauri::command]
pub async fn apply_text(
    app: AppHandle,
    state: State<'_, AppState>,
    operation_id: String,
    final_text: String,
) -> AppResult<()> {
    let operation = {
        let guard = state
            .inner
            .lock()
            .map_err(|_| AppError::OperationNotFound)?;
        guard
            .operations
            .get(&operation_id)
            .cloned()
            .ok_or(AppError::OperationNotFound)?
    };

    paste_text_into_target(
        operation.target,
        final_text,
        operation.clipboard_snapshot.clone(),
    )
    .await?;

    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|_| AppError::OperationNotFound)?;
        guard.operations.remove(&operation_id);
        if guard.active_operation_id.as_deref() == Some(operation_id.as_str()) {
            guard.active_operation_id = None;
        }
    }

    hide_popup(&app)?;
    Ok(())
}

#[tauri::command]
pub fn cancel_operation(
    app: AppHandle,
    state: State<'_, AppState>,
    operation_id: String,
) -> AppResult<()> {
    let operation = {
        let mut guard = state
            .inner
            .lock()
            .map_err(|_| AppError::OperationNotFound)?;
        let operation = guard
            .operations
            .remove(&operation_id)
            .ok_or(AppError::OperationNotFound)?;
        if guard.active_operation_id.as_deref() == Some(operation_id.as_str()) {
            guard.active_operation_id = None;
        }
        operation
    };

    let _ = clipboard::restore_clipboard(operation.clipboard_snapshot);
    hide_popup(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn start_manual_operation(app: AppHandle) -> AppResult<()> {
    handle_shortcut_trigger(app).await
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> AppResult<()> {
    show_window(&app, false)
}

#[tauri::command]
pub fn show_settings_window(app: AppHandle) -> AppResult<()> {
    show_window(&app, true)
}

#[tauri::command]
pub fn hide_to_tray(app: AppHandle) -> AppResult<()> {
    hide_popup(&app)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

pub async fn handle_shortcut_trigger(app: AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    {
        let guard = state
            .inner
            .lock()
            .map_err(|_| AppError::OperationNotFound)?;
        if guard.active_operation_id.is_some() {
            show_popup(&app)?;
            return Ok(());
        }
    }

    let target = window_state::capture_foreground_window()?;
    let (original_text, snapshot) = match copy_selection_from_active_window(target).await {
        Ok(result) => result,
        Err(error) => {
            let _ = emit_error(&app, error.user_message());
            let _ = show_popup(&app);
            return Err(error);
        }
    };

    if original_text.is_empty() {
        let _ = clipboard::restore_clipboard(snapshot);
        emit_error(&app, AppError::EmptySelection.user_message())?;
        show_popup(&app)?;
        return Err(AppError::EmptySelection);
    }

    let operation_id = Uuid::new_v4().to_string();
    let model = model_settings::get_model_settings(&app)?.selected_model;
    {
        let mut guard = state
            .inner
            .lock()
            .map_err(|_| AppError::OperationNotFound)?;
        guard.active_operation_id = Some(operation_id.clone());
        guard.operations.insert(
            operation_id.clone(),
            OperationContext {
                target,
                clipboard_snapshot: snapshot,
            },
        );
    }

    app.emit(
        "operation-started",
        OperationStartedPayload {
            operation_id,
            original_text,
            target_captured: true,
            model,
        },
    )
    .map_err(|_| AppError::Window)?;
    show_popup(&app)?;
    Ok(())
}

pub async fn copy_selection_from_active_window(
    target: CapturedTargetWindow,
) -> AppResult<(String, ClipboardSnapshot)> {
    let snapshot = clipboard::snapshot_clipboard()?;
    window_state::focus_window(target)?;

    let sentinel = format!(
        "__PRIVACY_TEXT_ASSISTANT_COPY_SENTINEL_{}__",
        Uuid::new_v4()
    );
    clipboard::write_clipboard_text(sentinel.clone())?;
    tokio::time::sleep(Duration::from_millis(40)).await;
    keyboard::send_ctrl_c()?;

    let mut elapsed_ms = 0;
    while elapsed_ms <= 700 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        elapsed_ms += 50;

        if let Ok(text) = clipboard::read_clipboard_text() {
            if text != sentinel {
                return Ok((text, snapshot));
            }
        }
    }

    let _ = clipboard::restore_clipboard(snapshot.clone());
    Ok((String::new(), snapshot))
}

pub async fn paste_text_into_target(
    target: CapturedTargetWindow,
    text: String,
    previous_clipboard: ClipboardSnapshot,
) -> AppResult<()> {
    clipboard::write_clipboard_text(text)?;
    window_state::focus_window(target)?;
    tokio::time::sleep(Duration::from_millis(80)).await;
    keyboard::send_ctrl_v()?;
    tokio::time::sleep(Duration::from_millis(180)).await;
    let _ = clipboard::restore_clipboard(previous_clipboard);
    Ok(())
}

fn show_popup(app: &AppHandle) -> AppResult<()> {
    show_window(app, false)
}

pub fn show_window(app: &AppHandle, settings: bool) -> AppResult<()> {
    let window = app.get_webview_window("main").ok_or(AppError::Window)?;
    window.center().map_err(|_| AppError::Window)?;
    window
        .set_always_on_top(false)
        .map_err(|_| AppError::Window)?;
    let _ = window.unminimize();
    window.show().map_err(|_| AppError::Window)?;
    window.set_focus().map_err(|_| AppError::Window)?;
    if settings {
        let _ = app.emit("show-settings", ());
    }
    Ok(())
}

fn hide_popup(app: &AppHandle) -> AppResult<()> {
    let window = app.get_webview_window("main").ok_or(AppError::Window)?;
    window.hide().map_err(|_| AppError::Window)?;
    Ok(())
}

fn emit_error(app: &AppHandle, message: &str) -> AppResult<()> {
    app.emit(
        "operation-error",
        OperationErrorPayload {
            message: message.to_string(),
        },
    )
    .map_err(|_| AppError::Window)
}
