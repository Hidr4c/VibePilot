use image::{DynamicImage, RgbaImage};
use std::sync::Arc;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, RECT, BOOL, LPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateCompatibleBitmap, DeleteDC, DeleteObject, GetDC,
    ReleaseDC, SelectObject, SRCCOPY, EnumDisplayMonitors, GetMonitorInfoW, HMONITOR, HDC,
    MONITORINFO,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowTextW, GetWindowRect,
    IsWindowVisible, SetForegroundWindow, BringWindowToTop,
    GetForegroundWindow,
};

use super::{WindowInfo, ScreenInfo, BoundingBox, WindowAnchorResult, ScreenCapturerTrait, is_window_title_match};

#[cfg(target_os = "windows")]
use super::DpiAwarenessScope;
#[cfg(target_os = "windows")]
use super::win32_helpers::{enum_windows_proc, is_same_or_child_process};


pub struct ScreenCapturer {
    pub(super) monitors: Arc<Vec<ScreenInfo>>,
    pub(super) windows: std::sync::Arc<std::sync::Mutex<Vec<WindowInfo>>>,
    pub(super) active_track: std::sync::Mutex<Option<(isize, String)>>,
}

impl Default for ScreenCapturer {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenCapturer {
    pub fn new() -> Self {
        let monitors = Self::detect_monitors();
        let windows = std::sync::Arc::new(std::sync::Mutex::new(Self::enumerate_windows()));
        ScreenCapturer {
            monitors: Arc::new(monitors),
            windows,
            active_track: std::sync::Mutex::new(None),
        }
    }

    fn detect_monitors() -> Vec<ScreenInfo> {
        #[cfg(target_os = "windows")]
        let _scope = DpiAwarenessScope::enter_per_monitor_v2();

        #[cfg(target_os = "windows")]
        {
            unsafe extern "system" fn monitor_enum_proc(
                hmonitor: HMONITOR,
                _hdc: HDC,
                _rect: *mut RECT,
                data: LPARAM,
            ) -> BOOL {
                let monitors = &mut *(data.0 as *mut Vec<ScreenInfo>);
                let mut info = MONITORINFO::default();
                info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                if GetMonitorInfoW(hmonitor, &mut info).as_bool() {
                    let bbox = BoundingBox::new(
                        info.rcMonitor.left,
                        info.rcMonitor.top,
                        info.rcMonitor.right - info.rcMonitor.left,
                        info.rcMonitor.bottom - info.rcMonitor.top,
                    );
                    let name = format!("Screen {}", monitors.len() + 1);
                    monitors.push(ScreenInfo::new(name, bbox));
                }
                BOOL(1)
            }

            let mut monitors = Vec::new();
            unsafe {
                let data_ptr = &mut monitors as *mut Vec<ScreenInfo> as isize;
                let _ = EnumDisplayMonitors(HDC::default(), None, Some(monitor_enum_proc), LPARAM(data_ptr));
            }

            if monitors.is_empty() {
                unsafe {
                    let cx = windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN);
                    let cy = windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN);
                    monitors.push(ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, cx, cy)));
                }
            }
            monitors
        }
        #[cfg(not(target_os = "windows"))]
        {
            vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))]
        }
    }

    fn enumerate_windows() -> Vec<WindowInfo> {
        #[cfg(target_os = "windows")]
        {
            let _scope = DpiAwarenessScope::enter_per_monitor_v2();

            let mut collector = Vec::new();
            let data_ptr = &mut collector as *mut Vec<WindowInfo> as isize;

            unsafe {
                let _ = EnumWindows(Some(enum_windows_proc), LPARAM(data_ptr));
            }

            collector
        }
        #[cfg(not(target_os = "windows"))]
        {
            Vec::new()
        }
    }
}

impl ScreenCapturerTrait for ScreenCapturer {
    fn refresh_windows(&self) {
        if let Ok(mut wins) = self.windows.lock() {
            *wins = Self::enumerate_windows();
        }
    }

    fn get_windows(&self) -> Vec<WindowInfo> {
        if let Ok(wins) = self.windows.lock() {
            wins.clone()
        } else {
            Vec::new()
        }
    }

    fn get_monitors(&self) -> Vec<ScreenInfo> {
        self.monitors.as_ref().clone()
    }

    fn capture_desktop(&self) -> Option<DynamicImage> {
        if self.monitors.is_empty() { return None; }
        let min_x = self.monitors.iter().map(|m| m.bbox.left).min().unwrap_or(0);
        let min_y = self.monitors.iter().map(|m| m.bbox.top).min().unwrap_or(0);
        let max_x = self.monitors.iter().map(|m| m.bbox.left + m.bbox.width).max().unwrap_or(0);
        let max_y = self.monitors.iter().map(|m| m.bbox.top + m.bbox.height).max().unwrap_or(0);
        let width = max_x - min_x;
        let height = max_y - min_y;
        if width <= 0 || height <= 0 { return None; }
        self.capture_bbox(min_x, min_y, width, height)
    }

    fn capture_window_by_title(&self, title: &str) -> Option<DynamicImage> {
        let wins_copy = if let Ok(wins) = self.windows.lock() {
            wins.clone()
        } else {
            Vec::new()
        };

        // 1. Try active_track first
        if let Ok(track_lock) = self.active_track.lock() {
            if let Some((hwnd_val, ref target_ref)) = *track_lock {
                if target_ref == title {
                    if let Some(win) = wins_copy.iter().find(|w| w.hwnd == hwnd_val) {
                        if win.bbox.width > 0 && win.bbox.height > 0 {
                            #[cfg(target_os = "windows")]
                            unsafe {
                                let hwnd = HWND(win.hwnd as *mut std::ffi::c_void);
                                let fg_hwnd = GetForegroundWindow();
                                if !is_same_or_child_process(fg_hwnd, hwnd) {
                                    let _ = SetForegroundWindow(hwnd);
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                }
                            }
                            return self.capture_bbox(
                                win.bbox.left,
                                win.bbox.top,
                                win.bbox.width,
                                win.bbox.height,
                            );
                        }
                    }
                }
            }
        }

        // 2. Fallback to title matching
        for win in wins_copy {
            if is_window_title_match(&win.title, title) {
                if win.bbox.width <= 0 || win.bbox.height <= 0 {
                    continue;
                }
                
                if let Ok(mut track_lock) = self.active_track.lock() {
                    *track_lock = Some((win.hwnd, title.to_string()));
                }

                #[cfg(target_os = "windows")]
                unsafe {
                    let hwnd = HWND(win.hwnd as *mut std::ffi::c_void);
                    let fg_hwnd = GetForegroundWindow();
                    if !is_same_or_child_process(fg_hwnd, hwnd) {
                        let _ = SetForegroundWindow(hwnd);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
                
                return self.capture_bbox(
                    win.bbox.left,
                    win.bbox.top,
                    win.bbox.width,
                    win.bbox.height,
                );
            }
        }

        self.capture_desktop()
    }

    fn capture_bbox(&self, x: i32, y: i32, width: i32, height: i32) -> Option<DynamicImage> {
        #[cfg(target_os = "windows")]
        {
            let _scope = DpiAwarenessScope::enter_per_monitor_v2();

            unsafe {
                let hdc_src = GetDC(HWND::default());
                if hdc_src.is_invalid() { return None; }
                let hdc_mem = CreateCompatibleDC(hdc_src);
                if hdc_mem.is_invalid() { ReleaseDC(HWND::default(), hdc_src); return None; }
                let hbitmap = CreateCompatibleBitmap(hdc_src, width, height);
                if hbitmap.is_invalid() { let _ = DeleteDC(hdc_mem); ReleaseDC(HWND::default(), hdc_src); return None; }
                SelectObject(hdc_mem, hbitmap);
                let _ = BitBlt(hdc_mem, 0, 0, width, height, hdc_src, x, y, SRCCOPY);

                // Draw the actual mouse cursor on the captured image
                {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        GetCursorInfo, GetIconInfo, DrawIconEx, CURSORINFO, CURSOR_SHOWING, DI_NORMAL, HICON,
                    };
                    use windows::Win32::Graphics::Gdi::DeleteObject;

                    let mut cursor_info = CURSORINFO {
                        cbSize: std::mem::size_of::<CURSORINFO>() as u32,
                        ..Default::default()
                    };
                    if GetCursorInfo(&mut cursor_info).is_ok() && (cursor_info.flags.0 & CURSOR_SHOWING.0) != 0 {
                        let cursor_x = cursor_info.ptScreenPos.x;
                        let cursor_y = cursor_info.ptScreenPos.y;

                        // Check if the cursor is within the bounding box
                        if cursor_x >= x && cursor_x < x + width && cursor_y >= y && cursor_y < y + height {
                            let local_x = cursor_x - x;
                            let local_y = cursor_y - y;

                            let mut icon_info = windows::Win32::UI::WindowsAndMessaging::ICONINFO::default();
                            if GetIconInfo(HICON(cursor_info.hCursor.0), &mut icon_info).is_ok() {
                                let draw_x = local_x - icon_info.xHotspot as i32;
                                let draw_y = local_y - icon_info.yHotspot as i32;

                                let _ = DrawIconEx(
                                    hdc_mem,
                                    draw_x,
                                    draw_y,
                                    HICON(cursor_info.hCursor.0),
                                    0,
                                    0,
                                    0,
                                    None,
                                    DI_NORMAL,
                                );

                                // Clean up bitmaps returned by GetIconInfo to avoid memory leaks
                                if !icon_info.hbmMask.is_invalid() {
                                    let _ = DeleteObject(icon_info.hbmMask);
                                }
                                if !icon_info.hbmColor.is_invalid() {
                                    let _ = DeleteObject(icon_info.hbmColor);
                                }
                            }
                        }
                    }
                }

                let mut bmi: windows::Win32::Graphics::Gdi::BITMAP = std::mem::zeroed();
                let _ = windows::Win32::Graphics::Gdi::GetObjectW(hbitmap, std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAP>() as i32, Some(&mut bmi as *mut _ as *mut std::ffi::c_void));

                let bm_width = bmi.bmWidth as usize;
                let bm_height = bmi.bmHeight as usize;
                let mut buffer = vec![0u8; bm_width * bm_height * 4];

                let bmi_header = windows::Win32::Graphics::Gdi::BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAPINFOHEADER>() as u32,
                    biWidth: bmi.bmWidth,
                    biHeight: -bmi.bmHeight,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                };
                let mut bmi_info = windows::Win32::Graphics::Gdi::BITMAPINFO { bmiHeader: bmi_header, bmiColors: [Default::default()] };

                let usage = windows::Win32::Graphics::Gdi::DIB_RGB_COLORS;
                let _ = windows::Win32::Graphics::Gdi::GetDIBits(
                    hdc_mem, hbitmap, 0, bm_height as u32,
                    Some(buffer.as_mut_ptr() as *mut std::ffi::c_void),
                    &mut bmi_info as *mut _,
                    usage,
                );

                let mut buffer_mut = buffer;
                for pixel in buffer_mut.chunks_exact_mut(4) {
                    let temp = pixel[0]; pixel[0] = pixel[2]; pixel[2] = temp;
                }

                let img: RgbaImage = image::ImageBuffer::from_raw(bm_width as u32, bm_height as u32, buffer_mut)?;
                ReleaseDC(HWND::default(), hdc_src);
                let _ = DeleteObject(hbitmap);
                let _ = DeleteDC(hdc_mem);
                let dyn_img = DynamicImage::ImageRgba8(img);
                Some(dyn_img)
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (x, y, width, height);
            Some(DynamicImage::ImageRgba8(image::ImageBuffer::new(width as u32, height as u32)))
        }
    }

    fn is_target_visible(&self, target_windows: &[String]) -> bool {
        if target_windows.is_empty() {
            return true;
        }

        let wins_copy = if let Ok(wins) = self.windows.lock() {
            wins.clone()
        } else {
            Vec::new()
        };

        // 1. Try active_track first
        if let Ok(track_lock) = self.active_track.lock() {
            if let Some((hwnd, ref target_ref)) = *track_lock {
                if target_windows.iter().any(|t| t == target_ref) {
                    if wins_copy.iter().any(|w| w.hwnd == hwnd) {
                        return true;
                    }
                }
            }
        }

        // 2. Fallback to title matching
        for target in target_windows {
            let target_lower = target.to_lowercase();
            if target == crate::config::ALL_SCREENS_KEY 
                || target_lower.contains("all screens") 
                || target_lower.contains("tous les")
                || target_lower.contains("desktop") 
            {
                return true;
            }
            for win in &wins_copy {
                if is_window_title_match(&win.title, target) {
                    if let Ok(mut track_lock) = self.active_track.lock() {
                        *track_lock = Some((win.hwnd, target.clone()));
                    }
                    return true;
                }
            }
        }
        
        false
    }

    /// Returns the title of the window currently in the foreground.
    fn get_foreground_window_title(&self) -> Option<String> {
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }
            let mut title_buf = [0u16; 512];
            let title_len = GetWindowTextW(hwnd, &mut title_buf);
            if title_len > 0 {
                Some(String::from_utf16_lossy(&title_buf[..title_len as usize]).trim().to_string())
            } else {
                None
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }

    /// Ensures the target window is in the foreground before capture.
    fn ensure_window_foreground(&self, title: &str) -> WindowAnchorResult {
        // Refresh the window list to get current state
        self.refresh_windows();

        // 1. Check if active_track matches target and is already focused
        if let Ok(track_lock) = self.active_track.lock() {
            if let Some((hwnd_val, ref target_ref)) = *track_lock {
                if target_ref == title {
                    #[cfg(target_os = "windows")]
                    unsafe {
                        let hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
                        let fg_hwnd = GetForegroundWindow();
                        if fg_hwnd.0 as isize == hwnd_val || is_same_or_child_process(fg_hwnd, hwnd) {
                            return WindowAnchorResult::AlreadyFocused;
                        }
                    }
                }
            }
        }

        // Check if target title matches the foreground window title as fallback
        if let Some(fg_title) = self.get_foreground_window_title() {
            if is_window_title_match(&fg_title, title) {
                #[cfg(target_os = "windows")]
                unsafe {
                    let fg_hwnd = GetForegroundWindow();
                    if !fg_hwnd.0.is_null() {
                        if let Ok(mut track_lock) = self.active_track.lock() {
                            *track_lock = Some((fg_hwnd.0 as isize, title.to_string()));
                        }
                    }
                }
                return WindowAnchorResult::AlreadyFocused;
            }
        }

        // Target is not in the foreground, try to find and focus it
        let wins_copy = if let Ok(wins) = self.windows.lock() {
            wins.clone()
        } else {
            return WindowAnchorResult::WindowNotFound;
        };

        // 2. Try active_track window if target matches
        if let Ok(track_lock) = self.active_track.lock() {
            if let Some((hwnd_val, ref target_ref)) = *track_lock {
                if target_ref == title {
                    if let Some(win) = wins_copy.iter().find(|w| w.hwnd == hwnd_val) {
                        #[cfg(target_os = "windows")]
                        unsafe {
                            let hwnd = HWND(win.hwnd as *mut std::ffi::c_void);
                            let fg_hwnd = GetForegroundWindow();
                            if !is_same_or_child_process(fg_hwnd, hwnd) {
                                let _ = SetForegroundWindow(hwnd);
                                let _ = BringWindowToTop(hwnd);
                                std::thread::sleep(std::time::Duration::from_millis(150));
                                return WindowAnchorResult::Refocused;
                            } else {
                                return WindowAnchorResult::AlreadyFocused;
                            }
                        }
                    }
                }
            }
        }

        // 3. Fallback to title matching
        for win in &wins_copy {
            if is_window_title_match(&win.title, title) {
                if let Ok(mut track_lock) = self.active_track.lock() {
                    *track_lock = Some((win.hwnd, title.to_string()));
                }
                #[cfg(target_os = "windows")]
                unsafe {
                    let hwnd = HWND(win.hwnd as *mut std::ffi::c_void);
                    let fg_hwnd = GetForegroundWindow();
                    if !is_same_or_child_process(fg_hwnd, hwnd) {
                        let _ = SetForegroundWindow(hwnd);
                        let _ = BringWindowToTop(hwnd);
                        std::thread::sleep(std::time::Duration::from_millis(150));
                        return WindowAnchorResult::Refocused;
                    } else {
                        return WindowAnchorResult::AlreadyFocused;
                    }
                }
            }
        }

        WindowAnchorResult::WindowNotFound
    }
}

