use crate::env_subst::substitute_env_vars;
use crate::migration::migrate_config;
use crate::schema::OpenClawConfig;
use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Configuration manager that handles loading, validation, and hot-reload.
#[derive(Clone)]
pub struct ConfigManager {
    inner: Arc<ConfigManagerInner>,
}

struct ConfigManagerInner {
    config: RwLock<Arc<OpenClawConfig>>,
    config_path: PathBuf,
}

impl ConfigManager {
    /// Load configuration from the given file path.
    pub fn load(config_path: impl AsRef<Path>) -> Result<Self> {
        let config_path = config_path.as_ref().to_path_buf();
        let config = load_config_from_file(&config_path)?;
        let config = Arc::new(config);

        Ok(Self {
            inner: Arc::new(ConfigManagerInner {
                config: RwLock::new(config),
                config_path,
            }),
        })
    }

    /// Load with default path (~/.openclaw/config.json5).
    pub fn load_default() -> Result<Self> {
        let config_path = default_config_path();
        if config_path.exists() {
            Self::load(&config_path)
        } else {
            info!("No config file found at {}, using defaults", config_path.display());
            let config = Arc::new(OpenClawConfig::default());
            Ok(Self {
                inner: Arc::new(ConfigManagerInner {
                    config: RwLock::new(config),
                    config_path,
                }),
            })
        }
    }

    /// Get a snapshot of the current configuration.
    pub fn get(&self) -> Arc<OpenClawConfig> {
        self.inner.config.read().clone()
    }

    /// Reload configuration from disk.
    pub fn reload(&self) -> Result<()> {
        let new_config = load_config_from_file(&self.inner.config_path)?;
        let mut guard = self.inner.config.write();
        *guard = Arc::new(new_config);
        info!("Configuration reloaded from {}", self.inner.config_path.display());
        Ok(())
    }

    /// Get the config file path.
    pub fn config_path(&self) -> &Path {
        &self.inner.config_path
    }
}

/// Load and parse a configuration file.
fn load_config_from_file(path: &Path) -> Result<OpenClawConfig> {
    info!("Loading configuration from {}", path.display());

    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    // Step 1: Environment variable substitution
    let substituted = substitute_env_vars(&raw);

    // Step 2: Parse as JSON5
    let mut value: serde_json::Value = json5::from_str(&substituted)
        .with_context(|| format!("Failed to parse config file as JSON5: {}", path.display()))?;

    // Step 3: Legacy migration
    let migrated = migrate_config(&mut value);
    if migrated {
        debug!("Config migration applied");
    }

    // Step 4: Deserialize into typed config
    let config: OpenClawConfig = serde_json::from_value(value)
        .with_context(|| "Failed to deserialize config into OpenClawConfig schema")?;

    info!("Configuration loaded successfully");
    Ok(config)
}

/// Return the default config file path.
pub fn default_config_path() -> PathBuf {
    let home = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    home.join(".openclaw").join("config.json5")
}

/// Get the OpenClaw data directory.
pub fn data_dir() -> PathBuf {
    let home = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    home.join(".openclaw")
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}
