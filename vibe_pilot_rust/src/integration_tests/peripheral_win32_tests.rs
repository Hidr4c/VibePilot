use crate::peripheral_controller::{PeripheralController, ScreenOffset, CoordinateMapping};
use crate::screen_capture::{ScreenCapturer, ScreenCapturerTrait};
use rand::Rng;


#[cfg(target_os = "windows")]
fn real_win32_move_mouse_absolute(x: i32, y: i32) {
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
fn real_win32_click_current_position() {
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

#[test]
#[ignore]
fn test_real_click_and_move() {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let capturer = ScreenCapturer::new();
    capturer.refresh_windows();

    let windows = capturer.get_windows();
    let browser_win = windows.iter().find(|w| {
        let t = w.title.to_lowercase();
        t.contains("chrome") || t.contains("firefox") || t.contains("brave") || t.contains("edge") || t.contains("opera")
    });

    if let Some(win) = browser_win {
        println!("Found active browser window for real test: '{}' at {:?}", win.title, win.bbox);
        
        let controller = PeripheralController::new(std::sync::Arc::new(capturer));
        
        let offsets = vec![ScreenOffset {
            titre: win.title.clone(),
            left: win.bbox.left,
            top: win.bbox.top,
            width: win.bbox.width,
            height: win.bbox.height,
            y_offset: win.bbox.top,
        }];

        // Generate a random relative coordinate safely within the window bounds (0.15 to 0.85)
        let mut rng = rand::thread_rng();
        let rx: f64 = rng.gen_range(0.15..0.85);
        let ry: f64 = rng.gen_range(0.15..0.85);

        let (abs_x, abs_y) = controller.compute_absolute_coordinates(rx, ry, &offsets);
        println!("Moving mouse to random window coordinate (rx={:.3}, ry={:.3}) -> absolute: ({}, {}) and performing click...", rx, ry, abs_x, abs_y);

        #[cfg(target_os = "windows")]
        {
            real_win32_move_mouse_absolute(abs_x, abs_y);
            std::thread::sleep(std::time::Duration::from_millis(300));
            
            // Verify that the mouse actually landed on the exact absolute coordinate
            let mut pt = windows::Win32::Foundation::POINT::default();
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
            }
            println!("Actual real mouse position after move: ({}, {})", pt.x, pt.y);
            assert_eq!(pt.x, abs_x, "Mouse cursor X coordinate mismatch!");
            assert_eq!(pt.y, abs_y, "Mouse cursor Y coordinate mismatch!");

            real_win32_click_current_position();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (abs_x, abs_y);
        }
    } else {
        println!("No open browser window found. Available windows: {:?}", 
            windows.iter().map(|w| &w.title).collect::<Vec<_>>());
    }
}

#[test]
#[ignore]
fn test_generate_visual_comparison_images() {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let capturer = ScreenCapturer::new();
    capturer.refresh_windows();

    let windows = capturer.get_windows();
    let browser_win = windows.iter().find(|w| {
        let t = w.title.to_lowercase();
        t.contains("chrome") || t.contains("firefox") || t.contains("brave") || t.contains("edge") || t.contains("opera")
    });

    if let Some(win) = browser_win {
        println!("Found active browser window for visual test: '{}' at {:?}", win.title, win.bbox);
        
        let controller = PeripheralController::new(std::sync::Arc::new(capturer));
        
        let offsets = vec![ScreenOffset {
            titre: win.title.clone(),
            left: win.bbox.left,
            top: win.bbox.top,
            width: win.bbox.width,
            height: win.bbox.height,
            y_offset: win.bbox.top,
        }];

        // Generate a random relative coordinate safely within the window bounds (0.15 to 0.85)
        let mut rng = rand::thread_rng();
        let rx: f64 = rng.gen_range(0.15..0.85);
        let ry: f64 = rng.gen_range(0.15..0.85);

        // 1. Capture clean window screenshot before mouse move
        let clean_capturer = ScreenCapturer::new();
        let clean_img = clean_capturer.capture_bbox(win.bbox.left, win.bbox.top, win.bbox.width, win.bbox.height)
            .expect("Failed to capture clean screenshot of target window");

        // 2. Generate target click location image
        let mut target_img = clean_img.clone();
        let green_color = image::Rgba([0, 255, 0, 255]); // Green for theoretical target
        crate::orchestrator::helpers::draw_click_marker(&mut target_img, rx, ry, &[green_color]);
        target_img.save("target_click_location.png").expect("Failed to save target_click_location.png");
        println!("Saved target_click_location.png (Green marker shows the theoretical target)");

        // 3. Move physical mouse to the calculated coordinate
        let (abs_x, abs_y) = controller.compute_absolute_coordinates(rx, ry, &offsets);
        println!("Moving physical mouse to coordinate: ({}, {})", abs_x, abs_y);
        #[cfg(target_os = "windows")]
        {
            real_win32_move_mouse_absolute(abs_x, abs_y);
            std::thread::sleep(std::time::Duration::from_millis(300));
            
            // 4. Capture the window again with the physical cursor drawn on it
            let click_capturer = ScreenCapturer::new();
            let click_img = click_capturer.capture_bbox(win.bbox.left, win.bbox.top, win.bbox.width, win.bbox.height)
                .expect("Failed to capture screenshot with mouse cursor");
            click_img.save("actual_mouse_click.png").expect("Failed to save actual_mouse_click.png");
            println!("Saved actual_mouse_click.png (Contains the drawn physical mouse cursor)");

            // 5. Generate superposed image (drawn physical cursor + theoretical green target)
            let mut superposed_img = click_img.clone();
            crate::orchestrator::helpers::draw_click_marker(&mut superposed_img, rx, ry, &[green_color]);
            superposed_img.save("superposed_comparison.png").expect("Failed to save superposed_comparison.png");
            println!("Saved superposed_comparison.png (Compare physical cursor and green target)");
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (abs_x, abs_y);
        }
    } else {
        println!("No open browser window found. Available windows: {:?}", 
            windows.iter().map(|w| &w.title).collect::<Vec<_>>());
    }
}
