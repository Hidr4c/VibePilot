use crate::peripheral_controller::{CoordinateMapping, ScreenOffset};
use super::concrete::PeripheralController;

impl CoordinateMapping for PeripheralController {
    fn compute_absolute_coordinates(&self, x_rel: f64, y_rel: f64, offsets: &[ScreenOffset]) -> (i32, i32) {
        if offsets.is_empty() {
            return (0, 0);
        }

        let mut x_rel = x_rel;
        let mut y_rel = y_rel;

        let is_pixel_or_thousandths = x_rel > 5.0 || y_rel > 5.0;

        if is_pixel_or_thousandths {
            if x_rel <= 1000.0 && y_rel <= 1000.0 && x_rel > 1.0 && y_rel > 1.0 {
                x_rel /= 1000.0;
                y_rel /= 1000.0;
            } else {
                if x_rel > 1.0 {
                    let target_w = offsets.first().map(|o| o.width).unwrap_or(1920) as f64;
                    x_rel /= target_w;
                }
                if y_rel > 1.0 {
                    let target_h = offsets.first().map(|o| o.height).unwrap_or(1080) as f64;
                    y_rel /= target_h;
                }
            }
        }

        if offsets.len() == 1 {
            let target = &offsets[0];
            let abs_x = target.left + (target.width as f64 * x_rel) as i32;
            let abs_y = target.top + (target.height as f64 * y_rel) as i32;
            return (abs_x, abs_y);
        }

        let is_desktop = offsets.len() > 1 || (offsets.len() == 1 && self.is_desktop_title(&offsets[0].titre));

        if is_desktop {
            let monitors = self.capturer.get_monitors();
            if monitors.is_empty() {
                return (0, 0);
            }

            let min_x = monitors.iter().map(|m| m.bbox.left).min().unwrap_or(0);
            let min_y = monitors.iter().map(|m| m.bbox.top).min().unwrap_or(0);
            let max_x = monitors.iter().map(|m| m.bbox.left + m.bbox.width).max().unwrap_or(0);
            let max_y = monitors.iter().map(|m| m.bbox.top + m.bbox.height).max().unwrap_or(0);

            let virtual_width = max_x - min_x;
            let virtual_height = max_y - min_y;

            if virtual_width <= 0 || virtual_height <= 0 {
                return (0, 0);
            }

            let abs_x = min_x + (virtual_width as f64 * x_rel) as i32;
            let abs_y = min_y + (virtual_height as f64 * y_rel) as i32;
            (abs_x, abs_y)
        } else {
            let total_height: i32 = offsets.iter().map(|o| o.height + 25).sum();
            let y_phys = (y_rel * total_height as f64) as i32;

            let mut target = &offsets[0];
            for offset in offsets {
                if offset.y_offset <= y_phys && y_phys < offset.y_offset + offset.height {
                    target = offset;
                    break;
                }
            }

            let y_local = y_phys - target.y_offset;
            let y_local_rel = y_local as f64 / target.height as f64;

            let abs_x = target.left + (target.width as f64 * x_rel) as i32;
            let abs_y = target.top + (target.height as f64 * y_local_rel) as i32;
            (abs_x, abs_y)
        }
    }

    fn is_desktop_title(&self, title: &str) -> bool {
        let t = title.to_lowercase();
        let t = t.replace(['é', 'è', 'ê', 'ë'], "e");
        t.contains("bureau")
            || t.contains("desktop")
            || t.contains("ecrans")
            || t.contains("screens")
            || t.contains("ecran")
            || t.contains("screen")
            || t.contains("all_screens")
            || t.contains("tous les")
            || t.contains("all screens")
    }

    fn get_screen_bounds(&self) -> (i32, i32, i32, i32) {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::{
                GetSystemMetrics, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
                SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_CXSCREEN, SM_CYSCREEN
            };
            unsafe {
                let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
                if width == 0 || height == 0 {
                    let cx = GetSystemMetrics(SM_CXSCREEN);
                    let cy = GetSystemMetrics(SM_CYSCREEN);
                    (0, 0, cx, cy)
                } else {
                    (left, top, left + width, top + height)
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            (0, 0, 1920, 1080)
        }
    }
}
