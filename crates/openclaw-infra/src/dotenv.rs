use std::path::Path;
use tracing::{debug, info};

/// Load .env files from the given directory and its parents.
pub fn load_dotenv(dir: &Path) {
    // Load from explicit path if .env exists
    let env_path = dir.join(".env");
    if env_path.exists() {
        match dotenvy::from_path(&env_path) {
            Ok(()) => info!("Loaded .env from {}", env_path.display()),
            Err(e) => debug!("Could not load .env from {}: {e}", env_path.display()),
        }
    }

    // Also try the standard dotenvy auto-search
    match dotenvy::dotenv() {
        Ok(path) => debug!("Also loaded .env from {}", path.display()),
        Err(_) => debug!("No additional .env file found via auto-search"),
    }
}
