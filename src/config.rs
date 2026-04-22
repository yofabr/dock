use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use time::UtcDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub archive_path: PathBuf,
    pub archives: Vec<Archive>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archive {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    #[serde(with = "time_serde")]
    pub created_at: UtcDateTime,
    pub size: u64,
    pub items_count: usize,
}

impl Archive {
    pub fn validate_name(name: &str) -> Result<String, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("Archive name cannot be empty".to_string());
        }
        if trimmed.len() > 64 {
            return Err("Archive name too long (max 64 characters)".to_string());
        }
        if trimmed.contains('/') || trimmed.contains('\\') {
            return Err("Archive name cannot contain path separators".to_string());
        }
        if trimmed == "." || trimmed == ".." {
            return Err("Archive name cannot be '.' or '..'".to_string());
        }
        if trimmed.starts_with('.') {
            return Err("Archive name cannot start with '.'".to_string());
        }
        if !trimmed.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err("Archive name can only contain letters, numbers, underscores, and hyphens".to_string());
        }
        Ok(trimmed.to_string())
    }
}

mod time_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::UtcDateTime;

    pub fn serialize<S>(date: &UtcDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(date.unix_timestamp())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<UtcDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = i64::deserialize(deserializer)?;
        Ok(UtcDateTime::from_unix_timestamp(timestamp).unwrap())
    }
}

impl Config {
    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dock")
            .join("config.json")
    }

    pub fn default_archive_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dock")
            .join("archives")
    }

    pub fn load_or_create() -> std::io::Result<Self> {
        let config_path = Self::default_config_path();

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = serde_json::from_str(&content).unwrap_or_else(|_| Config {
                archive_path: Self::default_archive_path(),
                archives: Vec::new(),
            });
            fs::create_dir_all(&config.archive_path)?;
            Ok(config)
        } else {
            let config = Config {
                archive_path: Self::default_archive_path(),
                archives: Vec::new(),
            };
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let config_path = Self::default_config_path();
        let archive_path = &self.archive_path;

        fs::create_dir_all(config_path.parent().unwrap_or(&config_path))?;
        fs::create_dir_all(archive_path)?;

        let content = serde_json::to_string_pretty(self).unwrap();
        fs::write(config_path, content)
    }
}

pub fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}", duration.as_nanos())
}
