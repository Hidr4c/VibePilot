use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use super::models::BootstrapConfig;

#[cfg(target_os = "windows")]
pub mod dpapi {
    use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB};
    use windows::Win32::Foundation::{LocalFree, HLOCAL};

    pub fn encrypt(data: &[u8]) -> Option<Vec<u8>> {
        let mut data_in = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut data_out = CRYPT_INTEGER_BLOB::default();

        unsafe {
            let res = CryptProtectData(
                &mut data_in,
                None,
                None,
                None,
                None,
                0x1, // CRYPTPROTECT_UI_FORBIDDEN
                &mut data_out,
            );

            if res.is_ok() && !data_out.pbData.is_null() {
                let result = std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec();
                let _ = LocalFree(HLOCAL(data_out.pbData as *mut std::ffi::c_void));
                Some(result)
            } else {
                None
            }
        }
    }

    pub fn decrypt(data: &[u8]) -> Option<Vec<u8>> {
        let mut data_in = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut data_out = CRYPT_INTEGER_BLOB::default();

        unsafe {
            let res = CryptUnprotectData(
                &mut data_in,
                None,
                None,
                None,
                None,
                0x1, // CRYPTPROTECT_UI_FORBIDDEN
                &mut data_out,
            );

            if res.is_ok() && !data_out.pbData.is_null() {
                let result = std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec();
                let _ = LocalFree(HLOCAL(data_out.pbData as *mut std::ffi::c_void));
                Some(result)
            } else {
                None
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod dpapi {
    pub fn encrypt(_data: &[u8]) -> Option<Vec<u8>> {
        None
    }
    pub fn decrypt(_data: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

pub fn load_secure_data<T: serde::de::DeserializeOwned>(path: &Path) -> Option<(T, bool)> {
    if let Ok(bytes) = fs::read(path) {
        if let Some(decrypted) = dpapi::decrypt(&bytes) {
            if let Ok(decrypted_str) = std::str::from_utf8(&decrypted) {
                if let Ok(data) = toml::from_str::<T>(decrypted_str) {
                    return Some((data, true));
                }
            }
        }
        if let Ok(bytes_str) = std::str::from_utf8(&bytes) {
            if let Ok(data) = toml::from_str::<T>(bytes_str) {
                return Some((data, false));
            }
        }
    }
    None
}

pub fn save_secure_data<T: serde::Serialize>(path: &Path, data: &T) -> bool {
    if let Ok(toml_str) = toml::to_string(data) {
        if let Some(encrypted) = dpapi::encrypt(toml_str.as_bytes()) {
            return fs::write(path, encrypted).is_ok();
        } else {
            return fs::write(path, toml_str.as_bytes()).is_ok();
        }
    }
    false
}

pub fn compress_blob<S, T>(data: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: Serialize,
{
    let serialized = rmp_serde::to_vec(data).map_err(serde::ser::Error::custom)?;
    let compressed = zstd::encode_all(&serialized[..], 3).map_err(serde::ser::Error::custom)?;
    serializer.serialize_bytes(&compressed)
}

pub fn decompress_blob<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let compressed = Vec::<u8>::deserialize(deserializer)?;
    let decompressed = zstd::decode_all(&compressed[..]).map_err(serde::de::Error::custom)?;
    rmp_serde::from_slice(&decompressed).map_err(serde::de::Error::custom)
}

pub fn contains_dangerous_command(text: &str) -> bool {
    let dangerous_keywords = [
        "format", "shutdown", "sudo", "rm -rf", "rm ", "drop table",
        "delete from", "chmod", "wget http", "curl http",
        "pip install", "npm install", "taskkill", "reg delete",
        "kill -9", "desactiver antivirus", "turn off firewall",
        "rmdir", "apt remove", "apt purge", "bcdedit", "dd if=",
    ];
    let lower = text.to_lowercase();
    dangerous_keywords.iter().any(|kw| lower.contains(kw))
}

/// Checks if a shortcut name corresponds to a dangerous key combination.
///
/// Dangerous combinations include window-closing, desktop-showing, and
/// system-level operations that could cause data loss.
pub fn is_dangerous_shortcut(name: &str) -> bool {
    let dangerous_shortcuts = [
        "close", "close_tab", "show_desktop", "minimize", "task_view",
        "force_quit", "kill_process", "shutdown",
    ];
    let lower = name.to_lowercase();
    dangerous_shortcuts.iter().any(|ds| ds == &lower)
}

/// Exports an decrypted audit log from an encrypted journal file.
///
/// Reads the encrypted journal, attempts to decrypt it, and writes
/// a human-readable JSON file to the specified output path.
pub fn export_decrypted_journal(encrypted_path: &std::path::Path, output_path: &std::path::Path) -> Result<(), String> {
    let encrypted_data = std::fs::read(encrypted_path)
        .map_err(|e| format!("Failed to read encrypted journal at {:?}: {}", encrypted_path, e))?;

    let decrypted_bytes = dpapi::decrypt(&encrypted_data)
        .ok_or_else(|| "Failed to decrypt journal — key may be unavailable".to_string())?;

    let decrypted_str = std::str::from_utf8(&decrypted_bytes)
        .map_err(|e| format!("Decrypted data is not valid UTF-8: {}", e))?;

    // Try to parse as JSON and re-serialize for pretty-printing
    let json_value: serde_json::Value = serde_json::from_str(decrypted_str)
        .unwrap_or(serde_json::Value::String(decrypted_str.to_string()));

    let pretty = serde_json::to_string_pretty(&json_value)
        .map_err(|e| format!("Failed to serialize decrypted journal: {}", e))?;

    std::fs::write(output_path, pretty.as_bytes())
        .map_err(|e| format!("Failed to write decrypted journal to {:?}: {}", output_path, e))?;

    Ok(())
}

pub fn get_bootstrap_config_path() -> PathBuf {
    if let Ok(val) = std::env::var("VIBEPILOT_BOOTSTRAP_PATH") {
        return PathBuf::from(val);
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    
    // Check if we are running inside cargo test
    let thread_name = std::thread::current().name().unwrap_or("main").to_string();
    if thread_name != "main" && (thread_name.contains("test") || std::env::args().any(|arg| arg.contains("test"))) {
        let safe_name = thread_name.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
            .map(|c| if c == ':' { '_' } else { c })
            .collect::<String>();
        exe_dir.join(format!("bootstrap_{}.enc", safe_name))
    } else {
        exe_dir.join("bootstrap.enc")
    }
}

pub fn load_bootstrap_config() -> BootstrapConfig {
    let path = get_bootstrap_config_path();
    if let Some((config, _)) = load_secure_data::<BootstrapConfig>(&path) {
        return config;
    }
    BootstrapConfig::default()
}

pub fn save_bootstrap_config(config: &BootstrapConfig) -> bool {
    let path = get_bootstrap_config_path();
    save_secure_data(&path, config)
}

/// Thread-safe persistent logger for VibePilot actions.
///
/// Writes timestamped log entries to a file in the storage directory.
/// Automatically rotates the log file when it exceeds `MAX_LOG_SIZE_BYTES`
/// (default: 5 MB), keeping one `.log.old` backup.
///
/// Does not include screenshots or sensitive data in log output.
pub struct ActionLogger {
    writer: std::sync::Mutex<Option<fs::File>>,
    log_path: PathBuf,
}

/// Maximum log file size in bytes before rotation (5 MB).
const MAX_LOG_SIZE_BYTES: u64 = 5 * 1024 * 1024;

impl ActionLogger {
    /// Creates a new logger writing to the given path.
    ///
    /// If the file cannot be created or opened, the logger will silently
    /// degrade — log calls will be no-ops instead of panicking.
    pub fn new(log_path: PathBuf) -> Self {
        let file = if log_path.exists() {
            fs::OpenOptions::new()
                .append(true)
                .open(&log_path)
                .or_else(|_| fs::File::create(&log_path))
                .ok()
        } else {
            fs::File::create(&log_path).ok()
        };
        Self {
            writer: std::sync::Mutex::new(file),
            log_path,
        }
    }

    /// Writes a structured log entry with timestamp, action, details, and result.
    pub fn log(&self, action: &str, detail: &str, result: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("[{}] Action: {} | Details: {} | Result: {}\n", timestamp, action, detail, result);
        self.write_entry(&entry);
    }

    /// Writes an info-level log entry.
    pub fn info(&self, message: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("[{}] INFO: {}\n", timestamp, message);
        self.write_entry(&entry);
    }

    /// Writes an error-level log entry.
    pub fn error(&self, message: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!("[{}] ERROR: {}\n", timestamp, message);
        self.write_entry(&entry);
    }

    /// Internal: writes to the log file, rotating if necessary.
    fn write_entry(&self, entry: &str) {
        use std::io::Write;
        if let Ok(mut guard) = self.writer.lock() {
            // Check rotation before writing
            self.maybe_rotate(&mut guard);

            if let Some(ref mut file) = *guard {
                let _ = file.write_all(entry.as_bytes());
                let _ = file.flush();
            }
        }
    }

    /// Rotates the log file if it exceeds `MAX_LOG_SIZE_BYTES`.
    fn maybe_rotate(&self, guard: &mut Option<fs::File>) {
        let should_rotate = self.log_path.exists()
            && fs::metadata(&self.log_path)
                .map(|m| m.len() >= MAX_LOG_SIZE_BYTES)
                .unwrap_or(false);

        if !should_rotate {
            return;
        }

        // Close the current file handle
        *guard = None;

        // Rotate: current -> .old
        let backup_path = self.log_path.with_extension("log.old");
        let _ = fs::remove_file(&backup_path);
        let _ = fs::rename(&self.log_path, &backup_path);

        // Open a fresh log file
        *guard = fs::File::create(&self.log_path).ok();
    }
}
