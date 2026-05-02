use crate::errors::{AppError, AppResult};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CapturedTargetWindow {
    pub hwnd: isize,
    pub captured_at_ms: u128,
}

pub fn capture_foreground_window() -> AppResult<CapturedTargetWindow> {
    capture_foreground_window_impl()
}

pub fn focus_window(target: CapturedTargetWindow) -> AppResult<()> {
    focus_window_impl(target)
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn capture_foreground_window_impl() -> AppResult<CapturedTargetWindow> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return Err(AppError::TargetWindowUnavailable);
    }

    Ok(CapturedTargetWindow {
        hwnd: hwnd.0 as isize,
        captured_at_ms: now_ms(),
    })
}

#[cfg(not(target_os = "windows"))]
fn capture_foreground_window_impl() -> AppResult<CapturedTargetWindow> {
    Err(AppError::TargetWindowUnavailable)
}

#[cfg(target_os = "windows")]
fn focus_window_impl(target: CapturedTargetWindow) -> AppResult<()> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, IsWindow, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    let hwnd = HWND(target.hwnd as *mut _);
    if !unsafe { IsWindow(hwnd).as_bool() } {
        return Err(AppError::TargetWindowUnavailable);
    }

    if unsafe { IsIconic(hwnd).as_bool() } {
        let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };
    }

    if !unsafe { SetForegroundWindow(hwnd).as_bool() } {
        return Err(AppError::TargetWindowUnavailable);
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn focus_window_impl(_target: CapturedTargetWindow) -> AppResult<()> {
    Err(AppError::TargetWindowUnavailable)
}
