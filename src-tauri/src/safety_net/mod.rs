use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedFile {
    pub id: String,
    pub original_path: String,
    pub snapshot_path: String,
    pub file_size: u64,
    pub agent_id: String,
    pub agent_name: String,
    pub action_type: String,
    pub created_at: String,
    pub restored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub success: bool,
    pub original_path: String,
    pub backup_of_current: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyNetStats {
    pub total_files: i64,
    pub files_today: i64,
    pub total_storage_bytes: u64,
    pub storage_limit_bytes: u64,
    pub oldest_snapshot: Option<String>,
    pub newest_snapshot: Option<String>,
    pub by_agent: HashMap<String, i64>,
    pub restored_today: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyNetSettings {
    pub max_storage_bytes: u64,
    pub retention_days: u32,
}

impl Default for SafetyNetSettings {
    fn default() -> Self {
        Self {
            max_storage_bytes: 524_288_000, // 500MB
            retention_days: 30,
        }
    }
}

const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5MB

const ALLOWED_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h", "rb", "php", "swift",
    "kt", "json", "yaml", "yml", "toml", "xml", "ini", "conf", "env", "md", "txt", "csv",
    "html", "css", "scss", "sh", "bash", "zsh", "ps1", "bat", "sql", "graphql", "vue", "svelte",
];

const DENIED_PATH_SEGMENTS: &[&str] = &[
    "node_modules/",
    ".git/objects/",
    "target/",
    "build/",
    "dist/",
    "__pycache__/",
    ".next/",
    ".turbo/",
    ".cache/",
];

const DENIED_FILENAMES: &[&str] = &[
    "package-lock.json",
    "Cargo.lock",
    "yarn.lock",
    "pnpm-lock.yaml",
];

const DENIED_TEMP_SUFFIXES: &[&str] = &[".swp", ".tmp"];

pub struct SafetyNetEngine {
    pub settings: SafetyNetSettings,
    storage_dir: PathBuf,
    current_storage_bytes: u64,
}

impl SafetyNetEngine {
    pub fn new(settings: SafetyNetSettings) -> Self {
        let storage_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".unalome")
            .join("safety-net");

        let _ = fs::create_dir_all(&storage_dir);

        // Calculate current storage size
        let current_storage_bytes = Self::calculate_dir_size(&storage_dir);

        Self {
            settings,
            storage_dir,
            current_storage_bytes,
        }
    }

    fn calculate_dir_size(dir: &Path) -> u64 {
        let mut size = 0u64;
        for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
        size
    }

    pub fn protect_file(
        &mut self,
        path: &str,
        agent_id: &str,
        agent_name: &str,
        action_type: &str,
    ) -> Option<ProtectedFile> {
        if !self.should_protect(path) {
            return None;
        }

        let source = Path::new(path);
        if !source.exists() || !source.is_file() {
            return None;
        }

        // Check file size
        let metadata = fs::metadata(source).ok()?;
        let file_size = metadata.len();
        if file_size > MAX_FILE_SIZE {
            return None;
        }

        // Read file content (verify it's readable)
        let content = fs::read(source).ok()?;

        let now = Utc::now();
        let date_dir = self.storage_dir.join(now.format("%Y-%m-%d").to_string());
        let _ = fs::create_dir_all(&date_dir);

        // Build snapshot filename: {stem}_{unix_ts}{ext}
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let snapshot_name = format!("{}_{}{}", stem, now.timestamp(), ext);
        let snapshot_path = date_dir.join(&snapshot_name);

        if fs::write(&snapshot_path, &content).is_err() {
            return None;
        }

        self.current_storage_bytes += file_size;

        let pf = ProtectedFile {
            id: Uuid::new_v4().to_string(),
            original_path: path.to_string(),
            snapshot_path: snapshot_path.to_string_lossy().to_string(),
            file_size,
            agent_id: agent_id.to_string(),
            agent_name: agent_name.to_string(),
            action_type: action_type.to_string(),
            created_at: now.to_rfc3339(),
            restored: false,
        };

        Some(pf)
    }

    pub fn should_protect(&self, path: &str) -> bool {
        let p = Path::new(path);

        // Check extension allowlist
        let ext = match p.extension().and_then(|e| e.to_str()) {
            Some(e) => e.to_lowercase(),
            None => return false,
        };
        if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
            return false;
        }

        // Check path denylist
        let path_str = path.replace('\\', "/");
        for denied in DENIED_PATH_SEGMENTS {
            if path_str.contains(denied) {
                return false;
            }
        }

        // Check filename denylist
        if let Some(filename) = p.file_name().and_then(|f| f.to_str()) {
            if DENIED_FILENAMES.contains(&filename) {
                return false;
            }
            // Check temp file patterns
            for suffix in DENIED_TEMP_SUFFIXES {
                if filename.ends_with(suffix) {
                    return false;
                }
            }
            if filename.ends_with('~') {
                return false;
            }
        }

        true
    }

    pub fn restore_file(&self, snapshot_path: &str, original_path: &str) -> RestoreResult {
        let snapshot = Path::new(snapshot_path);
        if !snapshot.exists() {
            return RestoreResult {
                success: false,
                original_path: original_path.to_string(),
                backup_of_current: None,
                message: "Snapshot file not found".to_string(),
            };
        }

        let original = Path::new(original_path);
        let mut backup_of_current = None;

        // If original exists, back it up first
        if original.exists() {
            let now = Utc::now();
            let stem = original
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let ext = original
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default();
            let backup_name = format!("pre-restore_{}_{}{}", stem, now.timestamp(), ext);
            let date_dir = self.storage_dir.join(now.format("%Y-%m-%d").to_string());
            let _ = fs::create_dir_all(&date_dir);
            let backup_path = date_dir.join(&backup_name);

            if let Err(e) = fs::copy(original, &backup_path) {
                return RestoreResult {
                    success: false,
                    original_path: original_path.to_string(),
                    backup_of_current: None,
                    message: format!("Failed to backup current file: {}", e),
                };
            }
            backup_of_current = Some(backup_path.to_string_lossy().to_string());
        }

        // Ensure parent directory exists
        if let Some(parent) = original.parent() {
            let _ = fs::create_dir_all(parent);
        }

        match fs::copy(snapshot, original) {
            Ok(_) => RestoreResult {
                success: true,
                original_path: original_path.to_string(),
                backup_of_current,
                message: "File restored successfully".to_string(),
            },
            Err(e) => RestoreResult {
                success: false,
                original_path: original_path.to_string(),
                backup_of_current,
                message: format!("Failed to restore file: {}", e),
            },
        }
    }

    pub fn preview_file(&self, snapshot_path: &str) -> String {
        let path = Path::new(snapshot_path);
        if !path.exists() {
            return "Snapshot not found".to_string();
        }

        match fs::read(path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => {
                    if text.len() > 5000 {
                        format!("{}...\n\n[Truncated at 5000 chars]", &text[..5000])
                    } else {
                        text
                    }
                }
                Err(_) => "Binary file".to_string(),
            },
            Err(e) => format!("Failed to read snapshot: {}", e),
        }
    }

    pub fn get_storage_size(&self) -> u64 {
        self.current_storage_bytes
    }

    pub fn storage_warning_needed(&self) -> bool {
        self.current_storage_bytes > (self.settings.max_storage_bytes * 80 / 100)
    }

    pub fn delete_snapshot_file(&mut self, snapshot_path: &str) -> bool {
        let path = Path::new(snapshot_path);
        if path.exists() {
            let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            if fs::remove_file(path).is_ok() {
                self.current_storage_bytes = self.current_storage_bytes.saturating_sub(size);
                return true;
            }
        }
        false
    }

    pub fn update_settings(&mut self, settings: SafetyNetSettings) {
        self.settings = settings;
    }
}
