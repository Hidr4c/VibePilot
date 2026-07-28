//! Persistent store with encryption, zstd compression, atomic write-back cache, and background persistence.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use rand::RngCore;
use serde::{Serialize, Deserialize};
use std::io::Write;
use std::fs;

/// Trait for structs that support enabling/disabling Windows DPAPI hardware encryption.
pub trait DpapiConfigurable {
    fn is_dpapi_enabled(&self) -> bool;
}

/// Inner struct that holds the actual state and configurations.
struct SecureStoreInner<T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + DpapiConfigurable + 'static> {
    path: PathBuf,
    data: Mutex<T>,
    dirty: Mutex<bool>,
    last_save: Mutex<Instant>,
    interval: Mutex<Duration>,
    cipher: Aes256Gcm,
}

/// Thread-safe wrapper handle for SecureStoreInner.
pub struct SecureStore<T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + DpapiConfigurable + 'static> {
    inner: Arc<SecureStoreInner<T>>,
}

impl<T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + DpapiConfigurable + 'static> Clone for SecureStore<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + DpapiConfigurable + 'static> SecureStore<T> {
    /// Creates a new SecureStore. Loads existing data if file exists, or initializes with initial_data.
    pub fn new(path: PathBuf, initial_data: T, interval_secs: u64) -> Result<Self, String> {
        let parent_dir = path.parent()
            .ok_or_else(|| "Invalid secure store path".to_string())?;
        fs::create_dir_all(parent_dir).map_err(|e| e.to_string())?;

        // Derive 256-bit key from DPAPI (on Windows) or fallback on other platforms
        let key = get_key(parent_dir)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Cipher initialization failed: {}", e))?;

        let inner = Arc::new(SecureStoreInner {
            path: path.clone(),
            data: Mutex::new(initial_data),
            dirty: Mutex::new(false),
            last_save: Mutex::new(Instant::now()),
            interval: Mutex::new(Duration::from_secs(interval_secs)),
            cipher,
        });

        let store = Self { inner };

        if path.exists() {
            if let Err(e) = store.load() {
                if e.contains("Decryption failed") {
                    return Err(format!("Decryption failed: the data might be encrypted with a different key or keyring is inaccessible. Details: {}", e));
                }
                eprintln!("⚠️ Secure store load failed, renaming to .corrupt and starting fresh: {}", e);
                let corrupt_path = path.with_extension("corrupt");
                let _ = fs::rename(&path, &corrupt_path);
                // Save default initial state immediately to create valid file
                let _ = store.save_now();
            }
        } else {
            // First time: save default config immediately
            let _ = store.save_now();
        }

        // Start background persistence thread with Weak pointer to prevent Arc leak
        let weak_inner = Arc::downgrade(&store.inner);
        std::thread::spawn(move || loop {
            let interval = if let Some(inner) = weak_inner.upgrade() {
                *inner.interval.lock().expect("interval lock poisoned")
            } else {
                break;
            };

            std::thread::sleep(interval);

            if let Some(inner) = weak_inner.upgrade() {
                let dirty = *inner.dirty.lock().expect("dirty lock poisoned");
                let last = *inner.last_save.lock().expect("last_save lock poisoned");
                if dirty && last.elapsed() >= interval {
                    let _ = Self::save_now_inner(&inner);
                }
            } else {
                break;
            }
        });

        Ok(store)
    }

    /// Update store interval duration dynamically.
    pub fn set_interval(&self, interval: Duration) {
        if let Ok(mut lock) = self.inner.interval.lock() {
            *lock = interval;
        }
    }

    /// Read data from the store in a safe scoped block.
    pub fn read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let data = self.inner.data.lock().expect("SecureStore data mutex poisoned");
        f(&*data)
    }

    /// Modify the data and mark the store as dirty for background saving.
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut data = self.inner.data.lock().expect("SecureStore data mutex poisoned");
        f(&mut data);
        *self.inner.dirty.lock().expect("SecureStore dirty mutex poisoned") = true;
    }

    /// Force immediate save to disk.
    pub fn save_now(&self) -> Result<(), String> {
        Self::save_now_inner(&self.inner)
    }

    /// Loads and decrypts store from disk.
    pub fn load(&self) -> Result<(), String> {
        if !self.inner.path.exists() {
            return Ok(());
        }

        let mut bytes = fs::read(&self.inner.path)
            .map_err(|e| format!("Failed to read store file: {}", e))?;

        #[cfg(target_os = "windows")]
        {
            if let Some(decrypted) = crate::config::dpapi::decrypt(&bytes) {
                bytes = decrypted;
            }
        }

        if bytes.len() < 12 {
            return Err("File too small (missing nonce)".to_string());
        }

        let (nonce_bytes, ciphertext) = bytes.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let decrypted = self.inner.cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        // Deserialize MessagePack
        let data: T = rmp_serde::from_slice(&decrypted)
            .map_err(|e| format!("Deserialization failed: {}", e))?;

        // Store
        *self.inner.data.lock().expect("SecureStore data mutex poisoned") = data;

        Ok(())
    }

    fn save_now_inner(inner: &SecureStoreInner<T>) -> Result<(), String> {
        Self::save_now_inner_fields(
            &inner.path,
            &inner.data,
            &inner.dirty,
            &inner.last_save,
            &inner.cipher,
        )
    }

    fn save_now_inner_fields(
        path: &Path,
        data_mutex: &Mutex<T>,
        dirty_mutex: &Mutex<bool>,
        last_save_mutex: &Mutex<Instant>,
        cipher: &Aes256Gcm,
    ) -> Result<(), String> {
        let data = data_mutex.lock().unwrap();
        let dpapi_enabled = data.is_dpapi_enabled();
        
        // Serialize MessagePack
        let serialized = rmp_serde::to_vec(&*data)
            .map_err(|e| format!("Serialization failed: {}", e))?;

        // Random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt AES-256-GCM
        let encrypted = cipher.encrypt(nonce, serialized.as_slice())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let mut data_to_write = Vec::with_capacity(12 + encrypted.len());
        data_to_write.extend_from_slice(&nonce_bytes);
        data_to_write.extend_from_slice(&encrypted);

        #[cfg(target_os = "windows")]
        {
            if dpapi_enabled {
                if let Some(dpapi_encrypted) = crate::config::dpapi::encrypt(&data_to_write) {
                    data_to_write = dpapi_encrypted;
                } else {
                    return Err("Failed to encrypt data with DPAPI".to_string());
                }
            }
        }

        // Atomic write
        let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent_dir)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        temp.write_all(&data_to_write).map_err(|e| e.to_string())?;
        temp.flush().map_err(|e| e.to_string())?;

        temp.persist(path).map_err(|e| format!("Atomic write failed: {}", e))?;

        *dirty_mutex.lock().expect("SecureStore dirty mutex poisoned") = false;
        *last_save_mutex.lock().expect("SecureStore last_save mutex poisoned") = Instant::now();

        Ok(())
    }

}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    let mut res = Vec::with_capacity(s.len() / 2);
    for chunk in s.as_bytes().chunks(2) {
        let chunk_str = std::str::from_utf8(chunk).ok()?;
        let val = u8::from_str_radix(chunk_str, 16).ok()?;
        res.push(val);
    }
    Some(res)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub(crate) fn get_key(base_dir: &Path) -> Result<[u8; 32], String> {
    // Check if we are running in a test context.
    // We bypass keyring in tests to avoid global state pollution and CI issues.
    let is_test = cfg!(test)
        || std::env::var("VIBEPILOT_TEST").is_ok()
        || std::thread::current().name().unwrap_or("main").contains("test")
        || std::env::args().skip(1).any(|arg| arg.contains("test"));

    let stable_username = "VibePilot_Master_Key".to_string();
    let key_path = base_dir.join("key.enc");

    // 1. Try key.enc file first (DPAPI encrypted on Windows, raw on others)
    if key_path.exists() {
        if let Ok(encrypted_bytes) = fs::read(&key_path) {
            let mut key = [0u8; 32];
            let mut key_loaded = false;

            #[cfg(target_os = "windows")]
            {
                if let Some(decrypted) = crate::config::dpapi::decrypt(&encrypted_bytes) {
                    if decrypted.len() == 32 {
                        key.copy_from_slice(&decrypted);
                        key_loaded = true;
                    }
                }
            }

            if !key_loaded && encrypted_bytes.len() == 32 {
                key.copy_from_slice(&encrypted_bytes);
                key_loaded = true;
            }

            if key_loaded {
                return Ok(key);
            }
        }
    }

    // 2. Fallback to native keyring if key.enc doesn't exist
    if !is_test {
        // Try stable keyring entry
        if let Ok(entry) = keyring::Entry::new("VibePilot", &stable_username) {
            if let Ok(password) = entry.get_password() {
                if let Some(decoded) = hex_decode(&password) {
                    if decoded.len() == 32 {
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&decoded);
                        // Save back to key.enc for portability and sandbox compatibility
                        let _ = save_key_to_file(&key_path, &key);
                        return Ok(key);
                    }
                }
            }
        }

        // Try legacy path-based username
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(base_dir.to_string_lossy().as_bytes());
        let hash_hex: String = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect();
        let legacy_username = format!("EncryptionKey_{}", &hash_hex[..12]);

        if let Ok(legacy_entry) = keyring::Entry::new("VibePilot", &legacy_username) {
            if let Ok(password) = legacy_entry.get_password() {
                if let Some(decoded) = hex_decode(&password) {
                    if decoded.len() == 32 {
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&decoded);
                        // Save back to key.enc
                        let _ = save_key_to_file(&key_path, &key);
                        return Ok(key);
                    }
                }
            }
        }
    }

    // 3. Generate new key
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);

    // Save to file
    save_key_to_file(&key_path, &key)?;

    // Also attempt to save to stable keyring as an extra backup if not in test
    if !is_test {
        if let Ok(entry) = keyring::Entry::new("VibePilot", &stable_username) {
            let hex_pwd = hex_encode(&key);
            let _ = entry.set_password(&hex_pwd);
        }
    }

    Ok(key)
}

fn save_key_to_file(key_path: &Path, key: &[u8; 32]) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(encrypted) = crate::config::dpapi::encrypt(key) {
            fs::write(key_path, encrypted)
                .map_err(|e| format!("Failed to write encrypted key: {}", e))?;
        } else {
            return Err("Failed to encrypt key with DPAPI".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        fs::write(key_path, key)
            .map_err(|e| format!("Failed to write key: {}", e))?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(key_path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o600);
                let _ = fs::set_permissions(key_path, perms);
            }
        }
    }
    Ok(())
}


impl<T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + DpapiConfigurable + 'static> Drop for SecureStoreInner<T> {
    fn drop(&mut self) {
        let dirty = self.dirty.lock().ok().map(|d| *d).unwrap_or(false);
        if dirty {
            let _ = SecureStore::<T>::save_now_inner_fields(
                &self.path,
                &self.data,
                &self.dirty,
                &self.last_save,
                &self.cipher,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
    struct TestState {
        count: i32,
        name: String,
    }

    impl DpapiConfigurable for TestState {
        fn is_dpapi_enabled(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_secure_store_workflow() {
        let temp_dir = std::env::temp_dir().join("secure_store_test_dir");
        let _ = fs::create_dir_all(&temp_dir);
        let store_path = temp_dir.join("vibepilot_data.enc");
        let _ = fs::remove_file(&store_path);

        let initial = TestState { count: 42, name: "VibePilot".to_string() };
        let store = SecureStore::new(store_path.clone(), initial.clone(), 30).unwrap();

        // Check loaded matches initial
        store.read(|d| {
            assert_eq!(d.count, 42);
            assert_eq!(d.name, "VibePilot");
        });

        // Update data
        store.update(|d| {
            d.count = 100;
            d.name = "Updated".to_string();
        });

        // Force save now
        assert!(store.save_now().is_ok());

        // Create new store pointing to same file, check it loads updated data
        let store2 = SecureStore::new(store_path.clone(), initial.clone(), 30).unwrap();
        store2.read(|d| {
            assert_eq!(d.count, 100);
            assert_eq!(d.name, "Updated");
        });

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_secure_store_extra_coverage() {
        let temp_dir = std::env::temp_dir().join("secure_store_test_dir_extra");
        let _ = fs::create_dir_all(&temp_dir);
        let store_path = temp_dir.join("vibepilot_data_extra.enc");
        let _ = fs::remove_file(&store_path);

        let initial = TestState { count: 10, name: "VibePilotExtra".to_string() };
        let store = SecureStore::new(store_path.clone(), initial.clone(), 30).unwrap();

        // 1. Test set_interval
        store.set_interval(Duration::from_secs(10));
        assert_eq!(*store.inner.interval.lock().unwrap(), Duration::from_secs(10));

        // 2. Test corrupt file loading behavior (small file)
        fs::write(&store_path, b"short").unwrap();
        let res = store.load();
        assert!(res.is_err());

        // Constructing a new store on a corrupt file should succeed because it renames it and starts fresh
        let new_store = SecureStore::new(store_path.clone(), initial.clone(), 30);
        assert!(new_store.is_ok());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_secure_store_dpapi_toggle() {
        #[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
        struct TestStateDpapi {
            count: i32,
            dpapi: bool,
        }

        impl DpapiConfigurable for TestStateDpapi {
            fn is_dpapi_enabled(&self) -> bool {
                self.dpapi
            }
        }

        let temp_dir = std::env::temp_dir().join("secure_store_test_dpapi");
        let _ = fs::create_dir_all(&temp_dir);
        let store_path = temp_dir.join("vibepilot_data_dpapi.enc");
        let _ = fs::remove_file(&store_path);

        // 1. DPAPI enabled
        let initial1 = TestStateDpapi { count: 42, dpapi: true };
        let store1 = SecureStore::new(store_path.clone(), initial1.clone(), 30).unwrap();
        assert!(store1.save_now().is_ok());

        let store1_loaded = SecureStore::new(store_path.clone(), initial1.clone(), 30).unwrap();
        store1_loaded.read(|d| {
            assert_eq!(d.count, 42);
            assert_eq!(d.dpapi, true);
        });

        // 2. DPAPI disabled
        let _ = fs::remove_file(&store_path);
        let initial2 = TestStateDpapi { count: 99, dpapi: false };
        let store2 = SecureStore::new(store_path.clone(), initial2.clone(), 30).unwrap();
        assert!(store2.save_now().is_ok());

        let store2_loaded = SecureStore::new(store_path.clone(), initial2.clone(), 30).unwrap();
        store2_loaded.read(|d| {
            assert_eq!(d.count, 99);
            assert_eq!(d.dpapi, false);
        });

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

