use crate::orchestrator::loop_breaker::{LoopBreaker, LoopAction};
use crate::orchestrator::VibePilotOrchestrator;
use crate::event_bus::NotificationEvent;
use tokio::time::{sleep, Duration};

/// Heuristic mapping from window titles to executable names.
///
/// Uses partial matching on common application names.
pub fn find_executable_for_window(title: &str) -> Option<String> {
    let title_lower = title.to_lowercase();

    let mappings: &[(&[&str], &str)] = &[
        (&["chrome", "chromium"], "chrome.exe"),
        (&["firefox", "mozilla"], "firefox.exe"),
        (&["edge", "microsoft edge"], "msedge.exe"),
        (&["notepad++"], "notepad++.exe"),
        (&["notepad"], "notepad.exe"),
        (&["visual studio code", "vscode", " - code"], "code.exe"),
        (&["explorer", "file explorer", "explorateur"], "explorer.exe"),
        (&["word", "microsoft word"], "winword.exe"),
        (&["excel", "microsoft excel"], "excel.exe"),
        (&["powerpoint"], "powerpnt.exe"),
        (&["outlook"], "outlook.exe"),
        (&["discord"], "discord.exe"),
        (&["slack"], "slack.exe"),
        (&["spotify"], "spotify.exe"),
        (&["teams"], "teams.exe"),
        (&["obs studio", "obs"], "obs64.exe"),
        (&["vlc"], "vlc.exe"),
        (&["paint"], "mspaint.exe"),
        (&["calculator", "calculatrice"], "calc.exe"),
        (&["terminal", "windows terminal"], "wt.exe"),
        (&["powershell"], "powershell.exe"),
        (&["cmd", "command prompt", "invite de commandes"], "cmd.exe"),
    ];

    for (keywords, exe) in mappings {
        for keyword in *keywords {
            if title_lower.contains(keyword) {
                return Some(exe.to_string());
            }
        }
    }

    None
}

/// Helper function to check if the target API URL is local.
pub fn is_local_url(url: &str) -> bool {
    url.contains("localhost") || url.contains("127.0.0.1") || url.contains("::1")
}

/// Helper function to query local GPU utilization on Windows/Linux using nvidia-smi.
pub async fn get_gpu_utilization() -> Option<u32> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = tokio::process::Command::new("nvidia-smi");
        cmd.args(&["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
           .creation_flags(CREATE_NO_WINDOW);
        let output = tokio::time::timeout(std::time::Duration::from_secs(1), cmd.output()).await.ok()?.ok()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.trim().parse::<u32>().ok()
        } else {
            None
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = tokio::process::Command::new("nvidia-smi");
        cmd.args(&["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"]);
        let output = tokio::time::timeout(std::time::Duration::from_secs(1), cmd.output()).await.ok()?.ok()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.trim().parse::<u32>().ok()
        } else {
            None
        }
    }
}

/// Draws a highly visible red crosshair and circle at the specified relative coordinates on the screenshot.
pub fn draw_click_marker(img: &mut image::DynamicImage, rx: f64, ry: f64, colors: &[image::Rgba<u8>]) {
    if let Some(rgba) = img.as_mut_rgba8() {
        let (width, height) = rgba.dimensions();
        let px = (rx * width as f64).round() as i32;
        let py = (ry * height as f64).round() as i32;
        
        let num_colors = colors.len();
        if num_colors == 0 {
            return;
        }

        // The newest color is the last one in the slice
        let newest_color = colors[num_colors - 1];

        // Draw crosshair lines using the newest color
        let size = 12;
        for i in -size..=size {
            // Horizontal line
            let x = px + i;
            if x >= 0 && x < width as i32 && py >= 0 && py < height as i32 {
                rgba.put_pixel(x as u32, py as u32, newest_color);
            }
            // Vertical line
            let y = py + i;
            if px >= 0 && px < width as i32 && y >= 0 && y < height as i32 {
                rgba.put_pixel(px as u32, y as u32, newest_color);
            }
        }
        
        // Draw concentric rings from newest (innermost) to oldest (outermost)
        for (i, &color) in colors.iter().rev().enumerate() {
            let base_radius = 6 + i * 4;
            for radius in [base_radius, base_radius + 1] {
                let radius_i32 = radius as i32;
                for dy in -radius_i32..=radius_i32 {
                    for dx in -radius_i32..=radius_i32 {
                        let dist_sq = dx * dx + dy * dy;
                        if dist_sq >= (radius_i32 - 1) * (radius_i32 - 1) && dist_sq <= radius_i32 * radius_i32 {
                            let x = px + dx;
                            let y = py + dy;
                            if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                                rgba.put_pixel(x as u32, y as u32, color);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Parses key names into rdev::Key values.
pub fn parse_key_name(s: &str) -> Option<rdev::Key> {
    use rdev::Key;
    match s.to_lowercase().as_str() {
        "controlleft" | "ctrl" | "control" => Some(Key::ControlLeft),
        "controlright" => Some(Key::ControlRight),
        "altleft" | "alt" => Some(Key::Alt),
        "altright" | "altgr" => Some(Key::AltGr),
        "shiftleft" | "shift" => Some(Key::ShiftLeft),
        "shiftright" => Some(Key::ShiftRight),
        "meta" | "metaleft" | "win" | "command" | "super" => Some(Key::MetaLeft),
        "escape" | "esc" => Some(Key::Escape),
        "tab" => Some(Key::Tab),
        "return" | "enter" => Some(Key::Return),
        "space" => Some(Key::Space),
        "backspace" => Some(Key::Backspace),
        "delete" | "del" => Some(Key::Delete),
        "down" => Some(Key::DownArrow),
        "up" => Some(Key::UpArrow),
        "left" => Some(Key::LeftArrow),
        "right" => Some(Key::RightArrow),
        "keya" | "a" => Some(Key::KeyA),
        "keyb" | "b" => Some(Key::KeyB),
        "keyc" | "c" => Some(Key::KeyC),
        "keyd" | "d" => Some(Key::KeyD),
        "keye" | "e" => Some(Key::KeyE),
        "keyf" | "f" => Some(Key::KeyF),
        "keyg" | "g" => Some(Key::KeyG),
        "keyh" | "h" => Some(Key::KeyH),
        "keyi" | "i" => Some(Key::KeyI),
        "keyj" | "j" => Some(Key::KeyJ),
        "keyk" | "k" => Some(Key::KeyK),
        "keyl" | "l" => Some(Key::KeyL),
        "keym" | "m" => Some(Key::KeyM),
        "keyn" | "n" => Some(Key::KeyN),
        "keyo" | "o" => Some(Key::KeyO),
        "keyp" | "p" => Some(Key::KeyP),
        "keyq" | "q" => Some(Key::KeyQ),
        "keyr" | "r" => Some(Key::KeyR),
        "keys" | "s" => Some(Key::KeyS),
        "keyt" | "t" => Some(Key::KeyT),
        "keyu" | "u" => Some(Key::KeyU),
        "keyv" | "v" => Some(Key::KeyV),
        "keyw" | "w" => Some(Key::KeyW),
        "keyx" | "x" => Some(Key::KeyX),
        "keyy" | "y" => Some(Key::KeyY),
        "keyz" | "z" => Some(Key::KeyZ),
        _ => None,
    }
}

/// Helper function to handle LoopBreaker escape actions
pub async fn navigate_loop_breaker_actions(loop_breaker: &mut LoopBreaker, orchestrator: &VibePilotOrchestrator) {
    let loop_action = match loop_breaker.escalation_level {
        3 => LoopAction::SendEscapeKeys,
        4 => LoopAction::ForceScroll,
        5 => LoopAction::AlertUser,
        _ => LoopAction::Normal,
    };
    match loop_action {
        LoopAction::SendEscapeKeys => {
            orchestrator.bus.emit_notification(NotificationEvent::Log(
                "🔑 Loop escape: sending Escape + Tab keystrokes to break focus lock...".to_string()
            ));
            let _ = orchestrator.controller.key_combo(&[rdev::Key::Escape]);
            sleep(Duration::from_millis(150)).await;
            let _ = orchestrator.controller.key_combo(&[rdev::Key::Tab]);
            sleep(Duration::from_millis(300)).await;
        },
        LoopAction::ForceScroll => {
            orchestrator.bus.emit_notification(NotificationEvent::Log(
                "📜 Loop escape: forcing scroll down to change viewport...".to_string()
            ));
            let _ = orchestrator.controller.scroll_vertical(-3);
            sleep(Duration::from_millis(300)).await;
        },
        LoopAction::AlertUser => {
            let msg = format!(
                "L'orchestrateur a détecté {} actions identiques consécutives. Intervention requise.",
                loop_breaker.repetition_count()
            );
            orchestrator.bus.emit_notification(NotificationEvent::LoopDetectedAlert { message: msg.clone() });
            orchestrator.bus.emit_notification(NotificationEvent::Log(format!("🚨 LOOP ALERT: {}", msg)));
            // Auto-pause to let the user intervene
            orchestrator.bus.emit_notification(NotificationEvent::UpdateStatus {
                text: "LOOP DETECTED - Paused".to_string(),
                color: "red".to_string(),
            });
            sleep(Duration::from_secs(5)).await;
            loop_breaker.reset();
        },
        _ => {}
    }
}

/// Crops a 250x250 region centered around the relative coordinates, draws a target crosshair at center, and returns PNG bytes.
pub fn create_action_crop(img: &image::DynamicImage, rx: f64, ry: f64) -> Option<Vec<u8>> {
    let width = img.width();
    let height = img.height();
    let px = (rx * width as f64).round() as i32;
    let py = (ry * height as f64).round() as i32;
    
    // Crop area dimensions
    let crop_size = 250u32;
    let half_size = crop_size / 2;
    
    // Compute bounding box for crop, clamping to image boundaries
    let left = (px - half_size as i32).max(0).min(width as i32 - 1) as u32;
    let top = (py - half_size as i32).max(0).min(height as i32 - 1) as u32;
    
    let w = (crop_size).min(width - left);
    let h = (crop_size).min(height - top);
    
    if w == 0 || h == 0 {
        return None;
    }
    
    let mut crop = img.crop_imm(left, top, w, h);
    
    // Draw crosshair on the crop image. The target coordinates on the crop are (px - left, py - top)
    let target_rx = (px - left as i32) as f64 / w as f64;
    let target_ry = (py - top as i32) as f64 / h as f64;
    
    draw_click_marker(&mut crop, target_rx, target_ry, &[image::Rgba([255, 0, 0, 255])]);
    
    // Encode as PNG
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    if crop.write_to(&mut cursor, image::ImageFormat::Png).is_ok() {
        Some(buf)
    } else {
        None
    }
}

impl VibePilotOrchestrator {
    pub(crate) async fn cancelable_sleep(&self, duration: std::time::Duration) {
        if duration.is_zero() {
            return;
        }
        #[cfg(test)]
        {
            tokio::time::sleep(duration).await;
            return;
        }
        #[cfg(not(test))]
        {
            let mut remaining = duration;
            let step = std::time::Duration::from_millis(100);
            while remaining > std::time::Duration::ZERO {
                if self.bus.emit_query(crate::event_bus::QueryEvent::GetOrchestratorRunning) == "false" {
                    break;
                }
                let sleep_time = if remaining > step { step } else { remaining };
                tokio::time::sleep(sleep_time).await;
                remaining = remaining.saturating_sub(step);
            }
        }
    }
}

