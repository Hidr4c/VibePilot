/// DPI Awareness Context Scope helper to temporarily set thread DPI awareness on Windows.
#[cfg(target_os = "windows")]
pub struct DpiAwarenessScope {
    old_context: Option<windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT>,
}

#[cfg(target_os = "windows")]
impl DpiAwarenessScope {
    pub fn enter_per_monitor_v2() -> Self {
        use windows::Win32::UI::HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
        unsafe {
            let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            Self { old_context: Some(old) }
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for DpiAwarenessScope {
    fn drop(&mut self) {
        if let Some(old) = self.old_context {
            use windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext;
            unsafe {
                let _ = SetThreadDpiAwarenessContext(old);
            }
        }
    }
}
