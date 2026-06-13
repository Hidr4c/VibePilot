//! Incremental encrypted session journaling (append-only chiffré).

use std::path::PathBuf;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use rand::RngCore;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Types of journal entries stored sequentially.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum JournalEntryType {
    SessionStep(crate::orchestrator::ActionStep),
    SessionSnapshot(crate::config::SessionSnapshot),
    TaskGraphState(crate::memory::TaskGraph),
}

/// A timestamped entry in the journal.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JournalEntry {
    pub timestamp: DateTime<Utc>,
    pub entry_type: JournalEntryType,
}

/// Thread-safe, durable append-only encrypted journal storage.
pub struct AppendOnlyStore {
    path: PathBuf,
    file: Arc<Mutex<fs::File>>,
    cipher: Aes256Gcm,
}

impl Clone for AppendOnlyStore {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            file: self.file.clone(),
            cipher: self.cipher.clone(),
        }
    }
}

impl AppendOnlyStore {
    /// Opens or creates the append-only store at the given path.
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let parent = path.parent()
            .ok_or_else(|| "Invalid journal path".to_string())?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .map_err(|e| format!("Failed to open journal: {}", e))?;

        // Re-use same DPAPI key derivation
        let key = crate::secure_store::get_key(parent)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Cipher init failed: {}", e))?;

        Ok(Self {
            path,
            file: Arc::new(Mutex::new(file)),
            cipher,
        })
    }

    /// Appends a new serializable entry to the journal sequentially.
    pub fn append<T: Serialize>(&self, entry: &T) -> Result<(), String> {
        // Serialize MessagePack
        let serialized = rmp_serde::to_vec(entry)
            .map_err(|e| format!("Serialization failed: {}", e))?;

        // Generate nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt AES-256-GCM
        let ciphertext = self.cipher.encrypt(nonce, serialized.as_slice())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let len = ciphertext.len() as u32;
        let len_bytes = len.to_be_bytes();

        // Write sequentially
        let mut f = self.file.lock().unwrap();
        f.write_all(&nonce_bytes)
            .map_err(|e| format!("Failed to write nonce: {}", e))?;
        f.write_all(&len_bytes)
            .map_err(|e| format!("Failed to write length: {}", e))?;
        f.write_all(&ciphertext)
            .map_err(|e| format!("Failed to write ciphertext: {}", e))?;
        f.sync_all()
            .map_err(|e| format!("Failed to flush to disk: {}", e))?;

        Ok(())
    }

    /// Iterates over all journal entries, decrypting them. Skips corrupted blocks cleanly.
    pub fn iter_entries<T: for<'de> Deserialize<'de>>(&self) -> Result<Vec<T>, String> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let mut file = fs::File::open(&self.path)
            .map_err(|e| format!("Failed to open journal file for iteration: {}", e))?;

        let mut entries = Vec::new();

        loop {
            // Read nonce (12 bytes)
            let mut nonce_bytes = [0u8; 12];
            if let Err(e) = file.read_exact(&mut nonce_bytes) {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    break;
                }
                return Err(format!("Unexpected read error: {}", e));
            }
            let nonce = Nonce::from_slice(&nonce_bytes);

            // Read length (4 bytes)
            let mut len_bytes = [0u8; 4];
            if file.read_exact(&mut len_bytes).is_err() {
                eprintln!("⚠️ Journal read warning: unexpected EOF reading block length");
                break;
            }
            let len = u32::from_be_bytes(len_bytes) as usize;

            // Read ciphertext
            let mut ciphertext = vec![0u8; len];
            if file.read_exact(&mut ciphertext).is_err() {
                eprintln!("⚠️ Journal read warning: unexpected EOF reading block data");
                break;
            }

            // Decrypt GCM
            match self.cipher.decrypt(nonce, ciphertext.as_slice()) {
                Ok(plain) => {
                    // Deserialize MessagePack
                    match rmp_serde::from_slice::<T>(&plain) {
                        Ok(val) => {
                            entries.push(val);
                        }
                        Err(e) => {
                            eprintln!("⚠️ Failed to deserialize journal entry block: {}", e);
                            // Skip corrupted deserialization but continue to next block
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️ AEAD Decryption verification failed for journal entry block (corrupted): {}", e);
                    // Skip decryption failure (corrupted signature/data) but continue to next block
                }
            }
        }

        Ok(entries)
    }

    /// Retain only entries newer than max_age_days. Rewrites file atomically.
    pub fn retain_recent(&self, max_age_days: i64) -> Result<usize, String> {
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days);

        // Read all entries first
        let all_entries: Vec<JournalEntry> = self.iter_entries()?;
        let original_count = all_entries.len();

        let to_keep: Vec<JournalEntry> = all_entries.into_iter()
            .filter(|entry| entry.timestamp >= cutoff)
            .collect();
        let kept_count = to_keep.len();

        let removed_count = original_count - kept_count;
        if removed_count == 0 {
            return Ok(0); // Nothing to clean
        }

        // Rewrite atomically using tempfile
        let temp_path = self.path.with_extension("tmpclean");
        
        let new_store = AppendOnlyStore::new(temp_path.clone())?;
        for entry in &to_keep {
            new_store.append(entry)?;
        }

        // Close the temp file by dropping new_store
        drop(new_store);

        // Atomic rename
        fs::rename(&temp_path, &self.path)
            .map_err(|e| format!("Failed to atomically replace journal: {}", e))?;

        // Re-open our file handle pointing to the newly written file
        let mut file_guard = self.file.lock().unwrap();
        *file_guard = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&self.path)
            .map_err(|e| format!("Failed to re-open clean journal: {}", e))?;

        Ok(removed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_journal_append_and_read() {
        let temp_dir = std::env::temp_dir().join("journal_test_dir");
        let _ = fs::create_dir_all(&temp_dir);
        let path = temp_dir.join("session_journal.enc");
        let _ = fs::remove_file(&path);

        let store = AppendOnlyStore::new(path.clone()).unwrap();

        let step = crate::orchestrator::ActionStep {
            timestamp: "10:00".to_string(),
            action_type: "CLICK_AND_TYPE".to_string(),
            coordinates: Some((0.5, 0.5)),
            text_typed: Some("hello".to_string()),
            llm_report: "Report test".to_string(),
            was_repeated: false,
        };

        let entry = JournalEntry {
            timestamp: Utc::now(),
            entry_type: JournalEntryType::SessionStep(step.clone()),
        };

        store.append(&entry).unwrap();

        let entries: Vec<JournalEntry> = store.iter_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].timestamp.to_rfc3339(), entry.timestamp.to_rfc3339());
        if let JournalEntryType::SessionStep(ref s) = entries[0].entry_type {
            assert_eq!(s.action_type, "CLICK_AND_TYPE");
            assert_eq!(s.text_typed.as_deref(), Some("hello"));
        } else {
            panic!("Expected SessionStep type");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_journal_corruption_resilience() {
        let temp_dir = std::env::temp_dir().join("journal_corruption_test_dir");
        let _ = fs::create_dir_all(&temp_dir);
        let path = temp_dir.join("session_journal.enc");
        let _ = fs::remove_file(&path);

        let store = AppendOnlyStore::new(path.clone()).unwrap();

        let entry1 = JournalEntry {
            timestamp: Utc::now(),
            entry_type: JournalEntryType::SessionSnapshot(crate::config::SessionSnapshot {
                timestamp: "10:00".to_string(),
                objective: "Task 1".to_string(),
            }),
        };
        let entry2 = JournalEntry {
            timestamp: Utc::now(),
            entry_type: JournalEntryType::SessionSnapshot(crate::config::SessionSnapshot {
                timestamp: "11:00".to_string(),
                objective: "Task 2".to_string(),
            }),
        };

        store.append(&entry1).unwrap();
        store.append(&entry2).unwrap();

        // Let's corrupt the file by reading it, finding the second entry and scrambling its bytes
        let mut data = fs::read(&path).unwrap();
        
        // Find second block. The first block has: 12 bytes nonce + 4 bytes length.
        // Let's extract length of first ciphertext.
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&data[12..16]);
        let len1 = u32::from_be_bytes(len_bytes) as usize;
        
        let start_of_block2 = 12 + 4 + len1;
        // Corrupt the ciphertext of block 2 (e.g. flip a bit inside block 2)
        if data.len() > start_of_block2 + 20 {
            data[start_of_block2 + 20] ^= 0xFF;
        }

        fs::write(&path, &data).unwrap();

        // Reading entries again. The second one is corrupted, so we should skip it and get only the first one
        let entries: Vec<JournalEntry> = store.iter_entries().unwrap();
        assert_eq!(entries.len(), 1);
        if let JournalEntryType::SessionSnapshot(ref s) = entries[0].entry_type {
            assert_eq!(s.objective, "Task 1");
        } else {
            panic!("Expected SessionSnapshot");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_journal_retain_recent() {
        let temp_dir = std::env::temp_dir().join("journal_retain_test_dir");
        let _ = fs::create_dir_all(&temp_dir);
        let path = temp_dir.join("session_journal.enc");
        let _ = fs::remove_file(&path);

        let store = AppendOnlyStore::new(path.clone()).unwrap();

        let old_time = Utc::now() - chrono::Duration::days(10);
        let new_time = Utc::now();

        let old_entry = JournalEntry {
            timestamp: old_time,
            entry_type: JournalEntryType::SessionSnapshot(crate::config::SessionSnapshot {
                timestamp: "Old".to_string(),
                objective: "Old Task".to_string(),
            }),
        };
        let new_entry = JournalEntry {
            timestamp: new_time,
            entry_type: JournalEntryType::SessionSnapshot(crate::config::SessionSnapshot {
                timestamp: "New".to_string(),
                objective: "New Task".to_string(),
            }),
        };

        store.append(&old_entry).unwrap();
        store.append(&new_entry).unwrap();

        // Perform cleanup for older than 7 days
        let cleaned = store.retain_recent(7).unwrap();
        assert_eq!(cleaned, 1); // 1 entry cleaned

        // Read remaining
        let entries: Vec<JournalEntry> = store.iter_entries().unwrap();
        assert_eq!(entries.len(), 1);
        if let JournalEntryType::SessionSnapshot(ref s) = entries[0].entry_type {
            assert_eq!(s.objective, "New Task");
        } else {
            panic!("Expected SessionSnapshot");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
