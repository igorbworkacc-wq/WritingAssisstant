mod clipboard;
mod commands;
mod errors;
mod keyboard;
mod model_settings;
mod openai;
mod secure_store;
mod window_state;

use commands::{handle_shortcut_trigger, AppState};
use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::has_api_key,
            commands::set_api_key,
            commands::test_api_key,
            commands::get_api_key_status,
            commands::get_model_settings,
            commands::set_model_settings,
            commands::get_model_presets,
            commands::test_selected_model,
            commands::test_model,
            commands::run_correction,
            commands::run_rephrase,
            commands::apply_text,
            commands::cancel_operation,
            commands::start_manual_operation,
            commands::show_main_window,
            commands::show_settings_window,
            commands::hide_to_tray,
            commands::quit_app
        ])
        .setup(|app| {
            #[cfg(desktop)]
            setup_tray(app)?;

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
                            if triggered == &active_shortcut
                                && event.state() == ShortcutState::Pressed
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
                if crate::secure_store::has_api_key().unwrap_or(false) {
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
                let _ = window.set_always_on_top(false);
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(desktop)]
fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &hide, &quit])?;

    let mut builder = TrayIconBuilder::with_id("privacy-text-assistant")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("PrivacyTextAssistant")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                let _ = commands::show_window(app, false);
            }
            "settings" => {
                let _ = commands::show_window(app, true);
            }
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}
