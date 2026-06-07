//! Screen capture for VibePilot.

use image::{DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use windows::Win32::Foundation::{HWND, RECT, BOOL, LPARAM};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateCompatibleBitmap, DeleteDC, DeleteObject, GetDC,
    ReleaseDC, SelectObject, SRCCOPY, EnumDisplayMonitors, GetMonitorInfoW, HMONITOR, HDC,
    MONITORINFO,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowTextW, GetWindowRect,
    IsWindowVisible, SetForegroundWindow,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl BoundingBox {
    pub fn new(left: i32, top: i32, width: i32, height: i32) -> Self {
        BoundingBox { left, top, width, height }
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.left + self.width && y >= self.top && y < self.top + self.height
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub title: String,
    pub bbox: BoundingBox,
}

impl ScreenInfo {
    pub fn new(title: String, bbox: BoundingBox) -> Self {
        ScreenInfo { title, bbox }
    }
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub title: String,
    pub hwnd: isize,
    pub visible: bool,
    pub bbox: BoundingBox,
}


extern "system" fn enum_windows_proc(hwnd: HWND, data: LPARAM) -> BOOL {
    unsafe {
        let collector_ptr = data.0 as *mut Vec<WindowInfo>;
        if collector_ptr.is_null() {
            return BOOL(1);
        }
        let collector = &mut *collector_ptr;

        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        let mut class_name = [0u16; 256];
        let class_len = GetClassNameW(hwnd, &mut class_name);
        let class_str = String::from_utf16_lossy(&class_name[..class_len as usize]);

        if class_str.contains("XamlExplorerHostServer")
            || class_str.contains("Windows.UI.Core")
            || class_str.contains("ApplicationFrameWindow")
        {
            return BOOL(1);
        }

        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width < 100 || height < 100 {
            return BOOL(1);
        }

        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buf);
        let title = if title_len > 0 {
            String::from_utf16_lossy(&title_buf[..title_len as usize])
                .trim()
                .to_string()
        } else {
            format!("Window ({:x})", hwnd.0 as isize)
        };

        if !title.is_empty() {
            collector.push(WindowInfo {
                title,
                hwnd: hwnd.0 as isize,
                visible: true,
                bbox: BoundingBox::new(rect.left, rect.top, width, height),
            });
        }

        BOOL(1)
    }
}

pub struct ScreenCapturer {
    monitors: Arc<Vec<ScreenInfo>>,
    windows: std::sync::Arc<std::sync::Mutex<Vec<WindowInfo>>>,
}

impl ScreenCapturer {
    pub fn new() -> Self {
        let monitors = Self::detect_monitors();
        let windows = std::sync::Arc::new(std::sync::Mutex::new(Self::enumerate_windows()));
        ScreenCapturer { monitors: Arc::new(monitors), windows }
    }

    fn detect_monitors() -> Vec<ScreenInfo> {
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

    fn enumerate_windows() -> Vec<WindowInfo> {
        let mut collector = Vec::new();
        let data_ptr = &mut collector as *mut Vec<WindowInfo> as isize;

        unsafe {
            let _ = EnumWindows(Some(enum_windows_proc), LPARAM(data_ptr));
        }

        collector
    }

    pub fn refresh_windows(&self) {
        if let Ok(mut wins) = self.windows.lock() {
            *wins = Self::enumerate_windows();
        }
    }

    pub fn get_windows(&self) -> Vec<WindowInfo> {
        if let Ok(wins) = self.windows.lock() {
            wins.clone()
        } else {
            Vec::new()
        }
    }

    pub fn get_monitors(&self) -> Vec<ScreenInfo> {
        self.monitors.as_ref().clone()
    }

    pub fn capture_desktop(&self) -> Option<DynamicImage> {
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

    pub fn capture_window_by_title(&self, title: &str) -> Option<DynamicImage> {
        let title_lower = title.to_lowercase();
        
        let wins_copy = if let Ok(wins) = self.windows.lock() {
            wins.clone()
        } else {
            Vec::new()
        };
        for win in wins_copy {
            if win.title.to_lowercase().contains(&title_lower) || title_lower.contains(&win.title.to_lowercase()) {
                if win.bbox.width <= 0 || win.bbox.height <= 0 {
                    continue;
                }
                
                unsafe {
                    let hwnd = HWND(win.hwnd as *mut std::ffi::c_void);
                    let _ = SetForegroundWindow(hwnd);
                    std::thread::sleep(std::time::Duration::from_millis(100));
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

    pub fn capture_bbox(&self, x: i32, y: i32, width: i32, height: i32) -> Option<DynamicImage> {
        unsafe {
            let hdc_src = GetDC(HWND::default());
            if hdc_src.is_invalid() { return None; }
            let hdc_mem = CreateCompatibleDC(hdc_src);
            if hdc_mem.is_invalid() { ReleaseDC(HWND::default(), hdc_src); return None; }
            let hbitmap = CreateCompatibleBitmap(hdc_src, width, height);
            if hbitmap.is_invalid() { let _ = DeleteDC(hdc_mem); ReleaseDC(HWND::default(), hdc_src); return None; }
            SelectObject(hdc_mem, hbitmap);
            let _ = BitBlt(hdc_mem, 0, 0, width, height, hdc_src, x, y, SRCCOPY);

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
            let _ = dyn_img.save("e:\\Dev\\Projets\\Agent_AI\\last_capture.png");
            Some(dyn_img)
        }
    }

    pub fn is_target_visible(&self, target_titles: &[String]) -> bool {
        if target_titles.is_empty() { return true; }
        for target in target_titles {
            if target.to_lowercase().contains("all screens") || target.to_lowercase().contains("tous les") {
                return true;
            }
        }
        
        let wins_copy = if let Ok(wins) = self.windows.lock() {
            wins.clone()
        } else {
            Vec::new()
        };
        for target in target_titles {
            let target_lower = target.to_lowercase();
            for win in &wins_copy {
                if win.title.to_lowercase().contains(&target_lower) || target_lower.contains(&win.title.to_lowercase()) {
                    return true;
                }
            }
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_new_capturer() { let c = ScreenCapturer::new(); assert!(c.get_monitors().len() > 0); }
    #[test]
    fn test_bounding_box_contains() {
        let bbox = BoundingBox::new(100, 100, 200, 200);
        assert!(bbox.contains(150, 150));
        assert!(!bbox.contains(50, 50));
    }
    #[test]
    fn test_bounding_box_new() {
        let bbox = BoundingBox::new(0, 0, 1920, 1080);
        assert_eq!(bbox.left, 0); assert_eq!(bbox.width, 1920);
    }
}
