# PrivacyTextAssistant

PrivacyTextAssistant is a privacy-first Windows desktop writing assistant built with Tauri 2, React, TypeScript, and Rust.

It lets a user highlight text in another Windows application, press `Ctrl+Space`, review two OpenAI-powered alternatives, toggle individual changed tokens, and paste the selected final text back over the original selection.

## Privacy Design

- The frontend never calls OpenAI.
- The frontend never receives the raw OpenAI API key.
- API calls are made by the Rust backend directly to `https://api.openai.com`.
- API keys are stored through the OS credential store using the Rust `keyring` crate.
- `OPENAI_API_KEY` is supported as a development fallback.
- No telemetry, analytics, crash upload, remote logging, or hidden diagnostics are included.
- Selected text, prompts, responses, clipboard contents, and API keys are not logged.

## Configure the API Key

On first launch, if no key is available from Windows credential storage or `OPENAI_API_KEY`, the app shows an API-key setup screen.

You can also set a development key before launching:

```powershell
$env:OPENAI_API_KEY = "sk-..."
npm run tauri:dev
```

## Model Selection

The app uses the selected OpenAI API model for both correction and rephrase calls. The default model is `gpt-5-nano` unless changed in Settings.

`gpt-5-nano` is the preferred low-cost option for short writing tasks in this app. `gpt-5-mini` is available as a higher-capability cost-conscious option, `gpt-4o-mini` is retained as a legacy fallback preset, and `gpt-5` is available when output quality is more important than cost. A custom model ID can also be entered.

Some models may not be available to every API account. The model ID must exactly match an OpenAI API model ID. Pricing changes over time, so users should verify current OpenAI API pricing before selecting a model.

Model preference precedence:

1. User-selected model from persisted app settings.
2. `OPENAI_MODEL` environment variable.
3. Built-in default: `gpt-5-nano`.

The model ID is stored locally as a non-sensitive preference in the app config directory. The API key remains stored separately through secure storage.

## Run in Development

Install JavaScript dependencies:

```powershell
npm install
```

Install Rust and Cargo if they are not already available, plus the Visual C++ Build Tools with the Windows SDK:

```powershell
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools
```

Run the app:

```powershell
npm run tauri:dev
```

## Build for Windows

```powershell
npm run tauri:build
```

The Tauri config uses an NSIS current-user installer target so the app does not require administrative installation privileges in normal corporate Windows environments.

## Tests

Run frontend unit tests:

```powershell
npm test
```

The diff-token tests cover no-op changes, replacements, insertions, deletions, repeated words, punctuation, newlines, full revert, and full candidate reconstruction.

Rust tests are written for prompt construction and safe error messages, but require Rust/Cargo and the MSVC/Windows SDK toolchain on PATH:

```powershell
cd src-tauri
cargo test
```

## Keyboard Shortcut

Default global shortcut: `Ctrl+Space`

When pressed, the backend captures the foreground window, snapshots the text clipboard, simulates `Ctrl+C`, reads the selected text, and opens the review popup.

## Tray Behavior

PrivacyTextAssistant runs as a background utility with a system tray menu:

- Open: show the main window.
- Settings: show the API key/settings view.
- Hide: hide the window and keep the shortcut active.
- Quit: exit the app.

Closing the window hides it to the tray instead of terminating the app.

## Troubleshooting

- If `Ctrl+Space` does nothing, another application may already own that shortcut.
- If no popup result appears, make sure text is highlighted before pressing the shortcut.
- If OpenAI calls fail, replace the API key from the settings area or check `OPENAI_API_KEY`.
- Use the Test API key button to verify authentication without sending selected text.
- Use the Test model button to verify the selected model is available to the current API key.
- If paste fails, the original application window may have closed, lost focus eligibility, or blocked simulated paste.
- If clipboard restoration is imperfect, see `IMPLEMENTATION_NOTES.md` for clipboard format limitations.

## Known Limitations

- Clipboard preservation currently restores text clipboard contents only.
- Windows foreground focus rules can occasionally prevent refocusing another app.
- Some applications block synthetic `Ctrl+C` or `Ctrl+V`.
- Model availability depends on the current API account and is checked only when testing a model or making a request.
