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

## Troubleshooting

- If `Ctrl+Space` does nothing, another application may already own that shortcut.
- If no popup result appears, make sure text is highlighted before pressing the shortcut.
- If OpenAI calls fail, replace the API key from the settings area or check `OPENAI_API_KEY`.
- If paste fails, the original application window may have closed, lost focus eligibility, or blocked simulated paste.
- If clipboard restoration is imperfect, see `IMPLEMENTATION_NOTES.md` for clipboard format limitations.

## Known Limitations

- Clipboard preservation currently restores text clipboard contents only.
- Windows foreground focus rules can occasionally prevent refocusing another app.
- Some applications block synthetic `Ctrl+C` or `Ctrl+V`.
- The app uses the explicitly requested `gpt-4o-mini` model.
