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
}

/// Disk-based SessionReplayManager that retains up to 10 frames in memory.
pub struct FileSessionReplayManager {
    buffer: Mutex<VecDeque<DynamicImage>>,
    base_dir: PathBuf,
}

impl FileSessionReplayManager {
    /// Creates a new FileSessionReplayManager.
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(10)),
            base_dir,
        }
    }
}

impl SessionReplayManager for FileSessionReplayManager {
    fn record_frame(&self, img: &DynamicImage) {
        let mut queue = self.buffer.lock().unwrap();
        if queue.len() >= 10 {
            queue.pop_front();
        }
        queue.push_back(img.clone());
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
        let queue = self.buffer.lock().unwrap();
        for (idx, img) in queue.iter().enumerate() {
            let frame_path = report_dir.join(format!("frame_{:02}.png", idx + 1));
            let _ = img.save(frame_path);
        }

        Ok(report_dir)
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
