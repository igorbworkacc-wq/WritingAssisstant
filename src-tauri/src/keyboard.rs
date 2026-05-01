use crate::errors::{AppError, AppResult};

pub fn send_ctrl_c() -> AppResult<()> {
    send_ctrl_key('C')
}

pub fn send_ctrl_v() -> AppResult<()> {
    send_ctrl_key('V')
}

#[cfg(target_os = "windows")]
fn send_ctrl_key(key: char) -> AppResult<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
        VK_CONTROL,
    };

    let virtual_key = match key {
        'C' => VIRTUAL_KEY(0x43),
        'V' => VIRTUAL_KEY(0x56),
        _ => return Err(AppError::TargetWindowUnavailable),
    };

    let mut inputs = [
        keyboard_input(VK_CONTROL, false),
        keyboard_input(virtual_key, false),
        keyboard_input(virtual_key, true),
        keyboard_input(VK_CONTROL, true),
    ];

    let sent = unsafe {
        SendInput(
            &mut inputs,
            std::mem::size_of::<INPUT>() as i32,
        )
    };

    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err(AppError::TargetWindowUnavailable)
    }
}

#[cfg(target_os = "windows")]
fn keyboard_input(key: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY, up: bool) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    };

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(not(target_os = "windows"))]
fn send_ctrl_key(_key: char) -> AppResult<()> {
    Err(AppError::TargetWindowUnavailable)
}
