# Implementation Notes

## Architecture Deviations

The requested architecture was implemented directly:

- Tauri 2 desktop shell
- React + TypeScript frontend
- Rust backend commands
- OpenAI API calls from Rust only
- `keyring` secure storage
- `windows` crate for foreground window and focus handling
- Tauri global-shortcut plugin for `Ctrl+Space`
- `diff` / jsdiff for word-with-space diffing
- React reducer state for independent section and token state

No consumer ChatGPT web automation, telemetry, analytics, remote logging, or frontend OpenAI access was added.

## Clipboard Preservation

The backend snapshots and restores text clipboard contents at minimum. It intentionally does not attempt to preserve every native clipboard format, such as images, rich HTML, RTF, files, or application-private formats.

This limitation is documented because preserving all Windows clipboard formats safely requires enumerating and cloning native format handles, which is more invasive and riskier than text-only preservation. If the clipboard did not contain text before the operation, the app currently leaves non-text clipboard contents unchanged where possible but cannot fully restore them after writing paste text.

## Copy Detection

Before simulating `Ctrl+C`, the backend writes a random sentinel string to the clipboard after taking the snapshot. It then polls briefly for the active application to replace the sentinel with copied selection text. If the sentinel remains, the app treats the operation as an empty or failed selection and restores the snapshot.

The sentinel is never logged.

## Windows Focus And Paste Limitations

The app uses non-admin Win32 APIs and synthetic keyboard input. This avoids privileged hooks, drivers, services, and administrator-only APIs.

Windows may still refuse foreground focus changes depending on timing, user interaction state, elevated target processes, or application-specific behavior. Some corporate applications also block synthetic copy/paste. In those cases the app returns a safe user-facing error.

## API Key Security

API keys are saved via the OS credential store through `keyring`. The frontend only holds the key briefly in an input field before sending it to the backend command. It is cleared from React state immediately after save submission.

For development, `OPENAI_API_KEY` takes precedence if present.

## OpenAI API

The Rust backend calls the OpenAI Responses API endpoint directly with the backend-selected model and configured temperature.

The app sends text only to OpenAI and no other network service.

The Test API key button calls `GET https://api.openai.com/v1/models`, which validates authentication without sending selected user text.

## Model Selection

Model selection is backend-controlled. Correction and rephrase commands load the selected model from Rust settings at request start; the frontend cannot override the model per request.

The default model is `gpt-5-nano`. `gpt-4o-mini` remains as a fallback preset.

The selected model is persisted as non-sensitive local app configuration in `model-settings.json` under the Tauri app config directory. `OPENAI_MODEL` is used only when no persisted user-selected model exists, and `gpt-5-nano` remains the built-in default.

OpenAI model availability is validated only through the user-triggered test action or actual API calls. Pricing is not fetched automatically by the app.

The app retries once without `temperature` if the selected model rejects the temperature parameter.

## Tray And Window Behavior

The app creates a Tauri system tray icon with Open, Settings, Hide, and Quit actions. The main window starts hidden when an API key is configured and opens directly to settings when no key is configured.

The window is no longer permanently always-on-top. Closing the window hides it to the tray and keeps the global shortcut registered. The release executable uses the Windows GUI subsystem so it does not keep a console window open.

The crate-level `windows_subsystem = "windows"` attribute is placed in `src-tauri/src/main.rs` so release executables use the Windows GUI subsystem and avoid opening a console window.

## Packaging For Corporate Use

The Tauri configuration targets an NSIS current-user installer (`installMode: "currentUser"`). This is intended to avoid administrative installation privileges for ordinary use.

Portable packaging can be added later by enabling an additional Tauri Windows bundle target if the organization prefers uninstalled distribution.

## Build Environment Note

This workspace initially did not have `cargo` or `rustc` on PATH. Rustup was installed during implementation, and Cargo dependency download succeeded after setting `CARGO_HTTP_CHECK_REVOKE=false` for the corporate certificate-revocation environment.

Rust compilation is still blocked on this machine because the MSVC linker and Windows SDK import libraries are missing (`link.exe`, `kernel32.lib`, `userenv.lib`, and related libraries). The Visual Studio Build Tools installer returned Windows Installer exit code `1602`, so `cargo test` and `npm run tauri:build` could not be completed here.

Frontend tests and the frontend production build were run successfully.
