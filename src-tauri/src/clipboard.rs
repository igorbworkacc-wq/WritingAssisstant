use crate::errors::{AppError, AppResult};

#[derive(Clone, Debug)]
pub struct ClipboardSnapshot {
    pub text: Option<String>,
}

pub fn snapshot_clipboard() -> AppResult<ClipboardSnapshot> {
    let mut clipboard = arboard::Clipboard::new().map_err(|_| AppError::ClipboardUnavailable)?;
    let text = clipboard.get_text().ok();
    Ok(ClipboardSnapshot { text })
}

pub fn read_clipboard_text() -> AppResult<String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .map_err(|_| AppError::ClipboardUnavailable)
}

pub fn write_clipboard_text(text: String) -> AppResult<()> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text))
        .map_err(|_| AppError::ClipboardUnavailable)
}

pub fn restore_clipboard(snapshot: ClipboardSnapshot) -> AppResult<()> {
    if let Some(text) = snapshot.text {
        write_clipboard_text(text)?;
    }
    Ok(())
}
