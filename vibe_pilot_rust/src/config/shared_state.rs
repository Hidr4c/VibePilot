use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Trait defining shared state storage and exchange capabilities between nodes.
pub trait SharedStateManager: Send + Sync {
    /// Stores a text value associated with a key.
    fn store_value(&self, key: &str, value: &str) -> Result<(), String>;

    /// Retrieves a text value associated with a key.
    fn retrieve_value(&self, key: &str) -> Result<Option<String>, String>;

    /// Stores a binary file under a specific node namespace.
    fn store_file(&self, key: &str, file_content: &[u8], file_name: &str) -> Result<PathBuf, String>;

    /// Retrieves a binary file from a node namespace.
    fn retrieve_file(&self, key: &str, file_name: &str) -> Result<Option<Vec<u8>>, String>;
}

/// Disk-based implementation of SharedStateManager with a file-locking queue mechanism.
pub struct FileSharedStateManager {
    base_dir: PathBuf,
}

impl FileSharedStateManager {
    /// Creates a new FileSharedStateManager under the target base directory.
    pub fn new(base_dir: PathBuf) -> Self {
        let path = base_dir.join("vibe_shared");
        let _ = fs::create_dir_all(&path);
        Self { base_dir: path }
    }

    fn acquire_lock(&self, lock_path: &Path) -> Result<(), String> {
        let start = Instant::now();
        let timeout = Duration::from_secs(5);
        while start.elapsed() < timeout {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(lock_path)
            {
                Ok(_) => return Ok(()),
                Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(format!("Failed to create lock file: {}", e)),
            }
        }
        Err("Lock acquisition timed out".to_string())
    }

    fn release_lock(&self, lock_path: &Path) {
        let _ = fs::remove_file(lock_path);
    }
}

impl SharedStateManager for FileSharedStateManager {
    fn store_value(&self, key: &str, value: &str) -> Result<(), String> {
        let val_path = self.base_dir.join(format!("{}.val", key));
        let lock_path = self.base_dir.join(format!("{}.lock", key));
        self.acquire_lock(&lock_path)?;
        let res = fs::write(&val_path, value).map_err(|e| format!("Failed to write value: {}", e));
        self.release_lock(&lock_path);
        res
    }

    fn retrieve_value(&self, key: &str) -> Result<Option<String>, String> {
        let val_path = self.base_dir.join(format!("{}.val", key));
        let lock_path = self.base_dir.join(format!("{}.lock", key));
        if !val_path.exists() {
            return Ok(None);
        }
        self.acquire_lock(&lock_path)?;
        let res = fs::read_to_string(&val_path);
        self.release_lock(&lock_path);
        match res {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Failed to read value: {}", e)),
        }
    }

    fn store_file(&self, key: &str, file_content: &[u8], file_name: &str) -> Result<PathBuf, String> {
        let dir_path = self.base_dir.join(key);
        let _ = fs::create_dir_all(&dir_path);
        let target_path = dir_path.join(file_name);
        let lock_path = dir_path.join(format!("{}.lock", file_name));
        self.acquire_lock(&lock_path)?;
        let res = fs::write(&target_path, file_content).map_err(|e| format!("Failed to write file: {}", e));
        self.release_lock(&lock_path);
        res.map(|_| target_path)
    }

    fn retrieve_file(&self, key: &str, file_name: &str) -> Result<Option<Vec<u8>>, String> {
        let dir_path = self.base_dir.join(key);
        let target_path = dir_path.join(file_name);
        let lock_path = dir_path.join(format!("{}.lock", file_name));
        if !target_path.exists() {
            return Ok(None);
        }
        self.acquire_lock(&lock_path)?;
        let res = fs::read(&target_path);
        self.release_lock(&lock_path);
        match res {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Failed to read file: {}", e)),
        }
    }
}

/// Factory to construct SharedStateManager implementations following the IoC pattern.
pub struct SharedStateManagerFactory;

impl SharedStateManagerFactory {
    /// Creates a SharedStateManager instance.
    pub fn create(base_dir: PathBuf) -> std::sync::Arc<dyn SharedStateManager> {
        std::sync::Arc::new(FileSharedStateManager::new(base_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_shared_state_manager_values() {
        let dir = tempdir().unwrap();
        let manager = FileSharedStateManager::new(dir.path().to_path_buf());

        // Test non-existent value
        let val = manager.retrieve_value("missing").unwrap();
        assert!(val.is_none());

        // Test storing and retrieving
        manager.store_value("mykey", "hello state").unwrap();
        let val = manager.retrieve_value("mykey").unwrap();
        assert_eq!(val, Some("hello state".to_string()));
    }

    #[test]
    fn test_file_shared_state_manager_files() {
        let dir = tempdir().unwrap();
        let manager = FileSharedStateManager::new(dir.path().to_path_buf());

        // Test non-existent file
        let file_data = manager.retrieve_file("node1", "data.txt").unwrap();
        assert!(file_data.is_none());

        // Test storing and retrieving file
        let bytes = b"binary state data";
        let path = manager.store_file("node1", bytes, "data.txt").unwrap();
        assert!(path.exists());

        let file_data = manager.retrieve_file("node1", "data.txt").unwrap();
        assert_eq!(file_data, Some(bytes.to_vec()));
    }

    #[test]
    fn test_factory_creation() {
        let dir = tempdir().unwrap();
        let manager = SharedStateManagerFactory::create(dir.path().to_path_buf());
        manager.store_value("test", "value").unwrap();
        assert_eq!(manager.retrieve_value("test").unwrap(), Some("value".to_string()));
    }
}

