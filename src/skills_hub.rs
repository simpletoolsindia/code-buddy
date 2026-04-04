//! Skills Hub - Download and manage skills from agentskills.io and custom sources
//!
//! Provides skill discovery, installation, and management.
//! Compatible with agentskills.io open standard.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};

/// Validate a URL for SSRF risks
fn validate_url(url: &str) -> Result<url::Url> {
    let parsed = url::Url::parse(url)?;

    // Only allow https and http schemes
    match parsed.scheme() {
        "https" | "http" => {}
        scheme => anyhow::bail!("URL scheme '{}' is not allowed (only https/http permitted)", scheme),
    }

    // Check for internal/private IP ranges (SSRF prevention)
    if let Some(host) = parsed.host_str() {
        // Try to resolve the host and check IPs
        let addr_str = format!("{}:{}", host, parsed.port().unwrap_or(80));
        if let Ok(addrs) = addr_str.to_socket_addrs() {
            for addr in addrs {
                let ip = addr.ip();
                let is_internal = match ip {
                    IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified(),
                    IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
                };
                if is_internal {
                    anyhow::bail!(
                        "URL host '{}' resolves to internal IP range ({}), which is not permitted",
                        host, ip
                    );
                }
            }
        }
    }

    Ok(parsed)
}

/// Skill metadata (YAML frontmatter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub license: Option<String>,
    pub platforms: Option<Vec<String>>,
    pub prerequisites: Option<Prerequisites>,
    pub compatibility: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisites {
    pub env_vars: Option<Vec<String>>,
    pub commands: Option<Vec<String>>,
}

/// Skill installation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkill {
    pub path: PathBuf,
    pub metadata: SkillMetadata,
    pub content: String,
    pub installed_at: String,
}

/// Skills hub
pub struct SkillsHub {
    base_dir: PathBuf,
    registry: HashMap<String, InstalledSkill>,
}

impl SkillsHub {
    /// Create a new skills hub
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_dir)?;

        let mut hub = Self {
            base_dir: base_dir.clone(),
            registry: HashMap::new(),
        };
        hub.scan()?;
        Ok(hub)
    }

    /// Scan for installed skills
    pub fn scan(&mut self) -> Result<()> {
        self.registry.clear();

        if !self.base_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    if let Ok(skill) = self.load_skill_from_path(&path) {
                        self.registry.insert(skill.metadata.name.clone(), skill);
                    }
                }
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(skill) = self.load_skill_from_path(path.parent().unwrap()) {
                    self.registry.insert(skill.metadata.name.clone(), skill);
                }
            }
        }

        Ok(())
    }

    /// Load a skill from a path
    fn load_skill_from_path(&self, path: &Path) -> Result<InstalledSkill> {
        let skill_md = path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md)?;

        let (frontmatter, body) = extract_frontmatter(&content);
        let metadata = parse_frontmatter(&frontmatter)?;

        Ok(InstalledSkill {
            path: path.to_path_buf(),
            metadata,
            content: body,
            installed_at: chrono::Local::now().to_rfc3339(),
        })
    }

    /// Install a skill from URL
    pub async fn install_from_url(&mut self, url: &str) -> Result<()> {
        // SECURITY: Validate URL to prevent SSRF attacks
        let _validated = validate_url(url)?;
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        let content = response.text().await?;

        // Extract name from URL or content
        let name = url
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .trim_end_matches(".md")
            .to_string();

        let skill_dir = self.base_dir.join(&name);
        fs::create_dir_all(&skill_dir)?;

        let skill_path = skill_dir.join("SKILL.md");
        fs::write(&skill_path, &content)?;

        // Load and register
        let skill = self.load_skill_from_path(&skill_dir)?;
        self.registry.insert(skill.metadata.name.clone(), skill);

        Ok(())
    }

    /// Install a skill from agentskills.io
    pub async fn install_from_hub(&mut self, skill_name: &str) -> Result<()> {
        let url = format!("https://agentskills.io/skills/{}/SKILL.md", skill_name);
        self.install_from_url(&url).await
    }

    /// Remove a skill
    pub fn uninstall(&mut self, name: &str) -> Result<()> {
        if let Some(skill) = self.registry.remove(name) {
            if skill.path.exists() {
                fs::remove_dir_all(skill.path.parent().unwrap())?;
            }
        }
        Ok(())
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&InstalledSkill> {
        self.registry.get(name)
    }

    /// Get skill content
    pub fn get_content(&self, name: &str) -> Option<String> {
        self.registry.get(name).map(|s| s.content.clone())
    }

    /// List all skills
    pub fn list(&self) -> Vec<&InstalledSkill> {
        self.registry.values().collect()
    }

    /// Search skills by name or description
    pub fn search(&self, query: &str) -> Vec<&InstalledSkill> {
        let query_lower = query.to_lowercase();
        self.registry
            .values()
            .filter(|s| {
                s.metadata.name.to_lowercase().contains(&query_lower)
                    || s.metadata.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Format skills list as markdown
    pub fn format_list(&self) -> String {
        let mut md = String::from("# Available Skills\n\n");

        let mut categories: HashMap<&str, Vec<&InstalledSkill>> = HashMap::new();

        for skill in self.registry.values() {
            let cat = skill.metadata.metadata
                .as_ref()
                .and_then(|m| m.get("hermes"))
                .and_then(|h| h.get("category"))
                .and_then(|c| c.as_str())
                .unwrap_or("general");
            categories.entry(cat).or_default().push(skill);
        }

        for (cat, skills) in categories {
            md.push_str(&format!("## {}\n\n", cat));
            for skill in skills {
                md.push_str(&format!(
                    "- **{}** - {}\n",
                    skill.metadata.name,
                    skill.metadata.description
                ));
            }
            md.push('\n');
        }

        md
    }
}

/// Extract YAML frontmatter from markdown
fn extract_frontmatter(content: &str) -> (String, String) {
    if let Some(stripped) = content.strip_prefix("---") {
        if let Some(end) = stripped.find("---") {
            let frontmatter = content[3..end + 6].to_string();
            let body = stripped[end + 3..].trim().to_string();
            return (frontmatter, body);
        }
    }
    (String::new(), content.to_string())
}

/// Parse YAML frontmatter
fn parse_frontmatter(frontmatter: &str) -> Result<SkillMetadata> {
    if frontmatter.is_empty() {
        return Ok(SkillMetadata {
            name: "unknown".to_string(),
            description: String::new(),
            version: None,
            license: None,
            platforms: None,
            prerequisites: None,
            compatibility: None,
            metadata: None,
        });
    }

    // Simple YAML parsing for frontmatter
    let mut name = String::new();
    let mut description = String::new();
    let mut version = None;
    let mut license = None;
    let mut platforms = None;

    for line in frontmatter.lines().skip(1) {
        let line = line.trim();
        if line.starts_with("name:") {
            name = line.trim_start_matches("name:").trim().to_string();
        } else if line.starts_with("description:") {
            description = line.trim_start_matches("description:").trim().to_string();
        } else if line.starts_with("version:") {
            version = Some(line.trim_start_matches("version:").trim().to_string());
        } else if line.starts_with("license:") {
            license = Some(line.trim_start_matches("license:").trim().to_string());
        } else if line.starts_with("platforms:") {
            platforms = Some(
                line.trim_start_matches("platforms:")
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .collect()
            );
        }
    }

    Ok(SkillMetadata {
        name: if name.is_empty() { "unknown".to_string() } else { name },
        description,
        version,
        license,
        platforms,
        prerequisites: None,
        compatibility: None,
        metadata: None,
    })
}

/// Browse agentskills.io catalog
pub async fn browse_hub_catalog() -> Result<String> {
    // Fetch skill catalog from agentskills.io
    let response = reqwest::get("https://agentskills.io/api/skills")
        .await
        .context("Failed to fetch skills catalog")?;

    let skills: Vec<serde_json::Value> = serde_json::from_str(&response.text().await?)?;

    let mut md = String::from("# Skills Hub - Available Skills\n\n");
    md.push_str("Browse and install skills from [agentskills.io](https://agentskills.io)\n\n");

    for skill in skills.iter().take(50) {
        let name = skill.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let description = skill.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let category = skill.get("category").and_then(|v| v.as_str()).unwrap_or("general");
        let downloads = skill.get("downloads").and_then(|v| v.as_i64()).unwrap_or(0);

        md.push_str(&format!(
            "- **{}** ({}) - {} [{} downloads]\n",
            name, category, description, downloads
        ));
    }

    md.push_str("\nInstall with: `/skills install <name>`\n");

    Ok(md)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontmatter_extraction() {
        let content = r#"---
name: test-skill
description: A test skill
---

# Test Skill

Some content here.
"#;
        let (fm, body) = extract_frontmatter(content);
        assert!(!fm.is_empty());
        assert!(body.contains("Test Skill"));
    }

    #[test]
    fn test_parse_frontmatter() {
        let fm = r#"---
name: test-skill
description: A test skill
version: 1.0.0
---"#;
        let meta = parse_frontmatter(fm).unwrap();
        assert_eq!(meta.name, "test-skill");
        assert_eq!(meta.description, "A test skill");
        assert_eq!(meta.version, Some("1.0.0".to_string()));
    }
}
