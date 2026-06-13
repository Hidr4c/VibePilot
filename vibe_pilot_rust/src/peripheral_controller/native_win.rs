//! Native Windows input simulation.

#![allow(unused_imports)]

#[cfg(target_os = "windows")]
#[cfg_attr(tarpaulin, skip)]
pub fn win32_move_mouse_absolute(x: i32, y: i32) {
    if cfg!(test) {
        return;
    }
    use crate::screen_capture::DpiAwarenessScope;
    let _scope = DpiAwarenessScope::enter_per_monitor_v2();

    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
        SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SetPhysicalCursorPos, SetCursorPos
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT
    };

    unsafe {
        if SetCursorPos(x, y).is_ok() {
            return;
        }
        if SetPhysicalCursorPos(x, y).is_ok() {
            return;
        }

        let min_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let min_y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let virtual_width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let virtual_height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        if virtual_width > 0 && virtual_height > 0 {
            let norm_x = ((x - min_x) * 65536) / virtual_width;
            let norm_y = ((y - min_y) * 65536) / virtual_height;

            let mut input = INPUT::default();
            input.r#type = INPUT_MOUSE;
            input.Anonymous.mi = MOUSEINPUT {
                dx: norm_x,
                dy: norm_y,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            };

            let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }
}

#[cfg(target_os = "windows")]
#[cfg_attr(tarpaulin, skip)]
pub fn win32_click_current_position() {
    if cfg!(test) {
        return;
    }
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT,
    };

    unsafe {
        let mut inputs = [INPUT::default(), INPUT::default()];

        inputs[0].r#type = INPUT_MOUSE;
        inputs[0].Anonymous.mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_LEFTDOWN,
            time: 0,
            dwExtraInfo: 0,
        };

        inputs[1].r#type = INPUT_MOUSE;
        inputs[1].Anonymous.mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_LEFTUP,
            time: 0,
            dwExtraInfo: 0,
        };

        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(target_os = "windows")]
#[cfg_attr(tarpaulin, skip)]
pub fn win32_right_click_current_position() {
    if cfg!(test) {
        return;
    }
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT,
    };
    unsafe {
        let mut inputs = [INPUT::default(), INPUT::default()];
        inputs[0].r#type = INPUT_MOUSE;
        inputs[0].Anonymous.mi = MOUSEINPUT {
            dx: 0, dy: 0, mouseData: 0,
            dwFlags: MOUSEEVENTF_RIGHTDOWN,
            time: 0, dwExtraInfo: 0,
        };
        inputs[1].r#type = INPUT_MOUSE;
        inputs[1].Anonymous.mi = MOUSEINPUT {
            dx: 0, dy: 0, mouseData: 0,
            dwFlags: MOUSEEVENTF_RIGHTUP,
            time: 0, dwExtraInfo: 0,
        };
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(target_os = "windows")]
#[cfg_attr(tarpaulin, skip)]
pub fn win32_middle_click_current_position() {
    if cfg!(test) {
        return;
    }
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEINPUT,
    };
    unsafe {
        let mut inputs = [INPUT::default(), INPUT::default()];
        inputs[0].r#type = INPUT_MOUSE;
        inputs[0].Anonymous.mi = MOUSEINPUT {
            dx: 0, dy: 0, mouseData: 0,
            dwFlags: MOUSEEVENTF_MIDDLEDOWN,
            time: 0, dwExtraInfo: 0,
        };
        inputs[1].r#type = INPUT_MOUSE;
        inputs[1].Anonymous.mi = MOUSEINPUT {
            dx: 0, dy: 0, mouseData: 0,
            dwFlags: MOUSEEVENTF_MIDDLEUP,
            time: 0, dwExtraInfo: 0,
        };
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn test_win32_simulations() {
        win32_move_mouse_absolute(100, 100);
        win32_click_current_position();
        win32_right_click_current_position();
        win32_middle_click_current_position();
    }
}

