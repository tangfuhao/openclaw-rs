use super::Skill;
use super::loader::load_skills;
use parking_lot::RwLock;
use std::path::PathBuf;
use tracing::info;

/// Manages skills loaded from the agent's workspace directory.
pub struct SkillManager {
    skills_dir: PathBuf,
    skills: RwLock<Vec<Skill>>,
}

impl SkillManager {
    pub fn new(skills_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            skills: RwLock::new(Vec::new()),
        }
    }

    /// Load or reload skills from disk.
    pub async fn refresh(&self) -> anyhow::Result<()> {
        let loaded = load_skills(&self.skills_dir).await?;
        info!(count = loaded.len(), "Skills loaded");
        *self.skills.write() = loaded;
        Ok(())
    }

    /// Get all enabled skills.
    pub fn get_enabled(&self) -> Vec<Skill> {
        self.skills
            .read()
            .iter()
            .filter(|s| s.enabled)
            .cloned()
            .collect()
    }

    /// Get a skill by name.
    pub fn get_by_name(&self, name: &str) -> Option<Skill> {
        self.skills
            .read()
            .iter()
            .find(|s| s.name == name)
            .cloned()
    }

    /// Build a skills summary for inclusion in the system prompt.
    pub fn build_prompt_section(&self) -> String {
        let skills = self.get_enabled();
        if skills.is_empty() {
            return String::new();
        }

        let mut section = String::from("# Available Skills\n\n");
        for skill in &skills {
            section.push_str(&format!("## {}\n", skill.name));
            if !skill.description.is_empty() {
                section.push_str(&format!("{}\n", skill.description));
            }
            section.push('\n');
        }
        section
    }
}
