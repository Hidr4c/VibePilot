#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, RECT, BOOL, LPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetWindowTextW, GetWindowRect, IsWindowVisible,
};
use super::{WindowInfo, BoundingBox};

#[cfg(target_os = "windows")]
pub(crate) extern "system" fn enum_windows_proc(hwnd: HWND, data: LPARAM) -> BOOL {
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

#[cfg(target_os = "windows")]
pub(crate) fn is_same_or_child_process(hwnd1: HWND, hwnd2: HWND) -> bool {
    if hwnd1.0.is_null() || hwnd2.0.is_null() {
        return false;
    }
    unsafe {
        let mut pid1 = 0u32;
        let mut pid2 = 0u32;
        let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd1, Some(&mut pid1));
        let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd2, Some(&mut pid2));
        
        if pid1 == 0 || pid2 == 0 {
            return false;
        }

        if pid1 == pid2 {
            return true;
        }

        // Compare process image/executable names to handle multi-process applications (like Chrome/Brave child processes)
        if let (Some(name1), Some(name2)) = (get_process_name(pid1), get_process_name(pid2)) {
            return name1 == name2;
        }

        false
    }
}

#[cfg(target_os = "windows")]
unsafe fn get_process_name(pid: u32) -> Option<String> {
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT};
    use windows::Win32::Foundation::CloseHandle;
    
    let handle_res = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
    if let Ok(handle) = handle_res {
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        let query_res = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR::from_raw(buffer.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        if query_res.is_ok() {
            let path = String::from_utf16_lossy(&buffer[..size as usize]);
            if let Some(filename) = std::path::Path::new(&path).file_name() {
                return Some(filename.to_string_lossy().to_lowercase());
            }
        }
    }
    None
}
