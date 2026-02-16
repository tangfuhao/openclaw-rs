pub mod loader;
pub mod workspace;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A skill definition loaded from the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String,
    pub path: PathBuf,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub enabled: bool,
}
