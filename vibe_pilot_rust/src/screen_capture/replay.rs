use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::fs;
use image::DynamicImage;

/// Trait to manage rolling session capture buffers for debugging/replay.
pub trait SessionReplayManager: Send + Sync {
    /// Adds a screenshot frame to the rolling in-memory buffer.
    fn record_frame(&self, img: &DynamicImage);

    /// Saves the rolling frames to a debug report directory on disk on failure.
    fn save_replay(&self, error_message: &str) -> Result<PathBuf, String>;

    /// Exports the rolling buffer as an animated GIF.
    fn export_gif(&self, path: &Path) -> Result<(), String>;

    /// Clears the full session frame history.
    fn clear_full_session(&self);

    /// Exports the entire recorded session as an animated GIF.
    fn export_full_session_gif(&self, path: &Path) -> Result<(), String>;

    /// Exports all screenshots, logs, and report to a directory.
    fn export_full_session_archive(
        &self,
        dir: &Path,
        logs: &[String],
        report: &str,
    ) -> Result<(), String>;
}

/// Disk-based SessionReplayManager that retains up to 10 frames in memory.
pub struct FileSessionReplayManager {
    buffer: Mutex<VecDeque<DynamicImage>>,
    full_buffer: Mutex<Vec<Vec<u8>>>,
    base_dir: PathBuf,
}

impl FileSessionReplayManager {
    /// Creates a new FileSessionReplayManager.
    pub fn new(base_dir: PathBuf) -> Self {
        let frames_dir = base_dir.join("session_frames");
        let _ = fs::remove_dir_all(&frames_dir);
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(10)),
            full_buffer: Mutex::new(Vec::new()),
            base_dir,
        }
    }
}

impl SessionReplayManager for FileSessionReplayManager {
    fn record_frame(&self, img: &DynamicImage) {
        // Record to rolling buffer
        {
            let mut queue = self.buffer.lock().unwrap();
            if queue.len() >= 10 {
                queue.pop_front();
            }
            queue.push_back(img.clone());
        }
        // Record to full session buffer (compressed JPEG in memory to avoid RAM saturation & SSD wear)
        {
            let mut full = self.full_buffer.lock().unwrap();
            let mut jpeg_bytes = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut jpeg_bytes);
            if img.write_to(&mut cursor, image::ImageFormat::Jpeg).is_ok() {
                full.push(jpeg_bytes);
            }
        }
    }

    fn save_replay(&self, error_message: &str) -> Result<PathBuf, String> {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let report_dir = self.base_dir.join(format!("debug_reports/report_{}", timestamp));
        fs::create_dir_all(&report_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

        // Save metadata info
        let info_path = report_dir.join("info.txt");
        let info_content = format!("Error: {}\nTimestamp: {}\n", error_message, timestamp);
        let _ = fs::write(&info_path, info_content);

        // Save rolling screenshots
        let has_frames = {
            let queue = self.buffer.lock().unwrap();
            for (idx, img) in queue.iter().enumerate() {
                let frame_path = report_dir.join(format!("frame_{:02}.png", idx + 1));
                let _ = img.save(frame_path);
            }
            !queue.is_empty()
        };

        // Also compile them into a replay.gif in the same directory!
        if has_frames {
            let gif_path = report_dir.join("replay.gif");
            let _ = self.export_gif(&gif_path);
        }

        Ok(report_dir)
    }

    fn export_gif(&self, path: &Path) -> Result<(), String> {
        let queue = self.buffer.lock().unwrap();
        if queue.is_empty() {
            return Err("No frames in replay buffer to export.".to_string());
        }

        let file = fs::File::create(path).map_err(|e| format!("Failed to create GIF file: {}", e))?;
        let mut encoder = image::codecs::gif::GifEncoder::new(file);

        for img in queue.iter() {
            let rgba_img = img.to_rgba8();
            let frame = image::Frame::from_parts(
                rgba_img,
                0,
                0,
                image::Delay::from_saturating_duration(std::time::Duration::from_millis(800)),
            );
            encoder.encode_frame(frame).map_err(|e| format!("Failed to encode GIF frame: {}", e))?;
        }

        Ok(())
    }

    fn clear_full_session(&self) {
        let mut full = self.full_buffer.lock().unwrap();
        full.clear();
    }

    fn export_full_session_gif(&self, path: &Path) -> Result<(), String> {
        let queue = self.full_buffer.lock().unwrap();
        if queue.is_empty() {
            return Err("No frames in full session buffer to export.".to_string());
        }

        let file = fs::File::create(path).map_err(|e| format!("Failed to create GIF file: {}", e))?;
        let mut encoder = image::codecs::gif::GifEncoder::new(file);

        for bytes in queue.iter() {
            let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to load frame from memory: {}", e))?;
            let rgba_img = img.to_rgba8();
            let frame = image::Frame::from_parts(
                rgba_img,
                0,
                0,
                image::Delay::from_saturating_duration(std::time::Duration::from_millis(800)),
            );
            encoder.encode_frame(frame).map_err(|e| format!("Failed to encode GIF frame: {}", e))?;
        }

        Ok(())
    }

    fn export_full_session_archive(
        &self,
        dir: &Path,
        logs: &[String],
        report: &str,
    ) -> Result<(), String> {
        fs::create_dir_all(dir).map_err(|e| format!("Failed to create directory: {}", e))?;

        // 1. Save logs.txt
        let logs_path = dir.join("logs.txt");
        let logs_content = logs.join("\n");
        fs::write(&logs_path, logs_content).map_err(|e| format!("Failed to write logs: {}", e))?;

        // 2. Save report.md
        let report_path = dir.join("report.md");
        fs::write(&report_path, report).map_err(|e| format!("Failed to write report: {}", e))?;

        // 3. Save all frames in full session buffer
        let frames_dir = dir.join("frames");
        fs::create_dir_all(&frames_dir).map_err(|e| format!("Failed to create frames directory: {}", e))?;

        let queue = self.full_buffer.lock().unwrap();
        for (idx, bytes) in queue.iter().enumerate() {
            let dest_path = frames_dir.join(format!("frame_{:03}.png", idx + 1));
            let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to decode frame for archiving: {}", e))?;
            img.save(&dest_path).map_err(|e| format!("Failed to save frame {}: {}", idx + 1, e))?;
        }

        if !queue.is_empty() {
            let gif_path = dir.join("replay.gif");
            let file = fs::File::create(gif_path).map_err(|e| format!("Failed to create GIF file: {}", e))?;
            let mut encoder = image::codecs::gif::GifEncoder::new(file);

            for bytes in queue.iter() {
                let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
                    .map_err(|e| format!("Failed to load frame from memory: {}", e))?;
                let rgba_img = img.to_rgba8();
                let frame = image::Frame::from_parts(
                    rgba_img,
                    0,
                    0,
                    image::Delay::from_saturating_duration(std::time::Duration::from_millis(800)),
                );
                encoder.encode_frame(frame).map_err(|e| format!("Failed to encode GIF frame: {}", e))?;
            }
        }

        Ok(())
    }
}

/// Factory to construct SessionReplayManager implementations following the IoC pattern.
pub struct SessionReplayFactory;

impl SessionReplayFactory {
    /// Creates a SessionReplayManager instance.
    pub fn create(base_dir: PathBuf) -> Arc<dyn SessionReplayManager> {
        Arc::new(FileSessionReplayManager::new(base_dir))
    }
}
