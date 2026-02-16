use super::Skill;
use std::path::Path;
use tracing::{debug, warn};

/// Load skills from a directory.
pub async fn load_skills(dir: &Path) -> anyhow::Result<Vec<Skill>> {
    let mut skills = Vec::new();

    if !dir.exists() {
        debug!("Skills directory does not exist: {}", dir.display());
        return Ok(skills);
    }

    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            match load_skill_file(&path).await {
                Ok(skill) => {
                    debug!(name = %skill.name, "Loaded skill");
                    skills.push(skill);
                }
                Err(e) => {
                    warn!("Failed to load skill from {}: {e}", path.display());
                }
            }
        }
    }

    Ok(skills)
}

/// Load a single skill from a markdown file with frontmatter.
async fn load_skill_file(path: &Path) -> anyhow::Result<Skill> {
    let content = tokio::fs::read_to_string(path).await?;

    let (name, description, tags, body) = parse_frontmatter(&content);

    let name = name.unwrap_or_else(|| {
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    Ok(Skill {
        name,
        description: description.unwrap_or_default(),
        content: body.to_string(),
        path: path.to_path_buf(),
        tags,
        enabled: true,
    })
}

/// Parse YAML frontmatter from markdown (--- delimited).
fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>, Vec<String>, &str) {
    if !content.starts_with("---") {
        return (None, None, Vec::new(), content);
    }

    if let Some(end) = content[3..].find("---") {
        let frontmatter = &content[3..3 + end];
        let body = &content[3 + end + 3..];

        let mut name = None;
        let mut description = None;
        let mut tags = Vec::new();

        for line in frontmatter.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("name:") {
                name = Some(val.trim().trim_matches('"').to_string());
            } else if let Some(val) = line.strip_prefix("description:") {
                description = Some(val.trim().trim_matches('"').to_string());
            } else if let Some(val) = line.strip_prefix("tags:") {
                tags = val
                    .trim()
                    .trim_matches(['[', ']'])
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        (name, description, tags, body.trim_start())
    } else {
        (None, None, Vec::new(), content)
    }
}
