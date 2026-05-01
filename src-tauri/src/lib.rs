mod clipboard;
mod commands;
mod errors;
mod keyboard;
mod openai;
mod secure_store;
mod window_state;

use commands::{handle_shortcut_trigger, AppState};
use std::sync::{Arc, Mutex};
use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::has_api_key,
            commands::set_api_key,
            commands::run_correction,
            commands::run_rephrase,
            commands::apply_text,
            commands::cancel_operation,
            commands::start_manual_operation
        ])
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{
                    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
                };

                let shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::Space);
                let active_shortcut = shortcut.clone();
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app_handle, triggered, event| {
                            if triggered == &active_shortcut && event.state() == ShortcutState::Pressed
                            {
                                let handle = app_handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    let _ = handle_shortcut_trigger(handle).await;
                                });
                            }
                        })
                        .build(),
                )?;
                app.global_shortcut().register(shortcut).map_err(|_| {
                    tauri::Error::Anyhow(anyhow::anyhow!(
                        "Shortcut registration failed. Ctrl+Space may already be in use."
                    ))
                })?;
            }

            if let Some(window) = app.get_webview_window("main") {
                if crate::secure_store::has_api_key() {
                    let _ = window.hide();
                } else {
                    let _ = window.center();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let state = window.state::<AppState>();
                if let Ok(mut guard) = state.inner.lock() {
                    if let Some(active_id) = guard.active_operation_id.clone() {
                        if let Some(operation) = guard.operations.remove(&active_id) {
                            let _ = clipboard::restore_clipboard(operation.clipboard_snapshot);
                        }
                    }
                    guard.active_operation_id = None;
                }
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub(crate) type Shared<T> = Arc<Mutex<T>>;
