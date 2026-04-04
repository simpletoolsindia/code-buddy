//! Plugin system for Code Buddy
//!
//! Plugins can extend Code Buddy with:
//! - Skills (slash commands)
//! - Hooks (pre/post execution)
//! - MCP servers
//! - Custom tools
//! - Commands
//! - Agents

use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// Plugin Manifest
// ============================================================================

/// Plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<PluginAuthor>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub skills: Vec<Skill>,
    #[serde(default)]
    pub hooks: HookConfig,
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
    #[serde(default)]
    pub commands: Vec<CommandDef>,
    #[serde(default)]
    pub agents: Vec<AgentDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct HookConfig {
    #[serde(default)]
    pub pre_tool_use: Vec<HookDef>,
    #[serde(default)]
    pub post_tool_use: Vec<HookDef>,
    #[serde(default)]
    pub post_tool_use_failure: Vec<HookDef>,
    #[serde(default)]
    pub permission_denied: Vec<HookDef>,
    #[serde(default)]
    pub permission_request: Vec<HookDef>,
    #[serde(default)]
    pub user_prompt_submit: Vec<HookDef>,
    #[serde(default)]
    pub session_start: Vec<HookDef>,
    #[serde(default)]
    pub session_end: Vec<HookDef>,
    #[serde(default)]
    pub stop: Vec<HookDef>,
    #[serde(default)]
    pub pre_compact: Vec<HookDef>,
    #[serde(default)]
    pub post_compact: Vec<HookDef>,
    #[serde(default)]
    pub file_changed: Vec<HookDef>,
    #[serde(default)]
    pub cwd_changed: Vec<HookDef>,
    #[serde(default)]
    pub setup: Vec<HookDef>,
    #[serde(default)]
    pub task_created: Vec<HookDef>,
    #[serde(default)]
    pub task_completed: Vec<HookDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDef {
    #[serde(rename = "type")]
    pub hook_type: HookType,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub matcher: Option<String>,
    #[serde(default)]
    pub if_cond: Option<String>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub async_exec: bool,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub allowed_env_vars: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum HookType {
    #[default]
    Command,
    Prompt,
    Agent,
    Http,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    #[serde(rename = "type")]
    pub server_type: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub argument_hint: Option<String>,
    #[serde(default)]
    pub user_invocable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub instructions: String,
}

// ============================================================================
// Skill
// ============================================================================

/// A skill is a slash command that expands to text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub when_to_use: Option<String>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub argument_hint: Option<String>,
    #[serde(default)]
    pub arguments: Vec<SkillArgument>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub user_invocable: bool,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillArgument {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

// ============================================================================
// Hook
// ============================================================================

/// A hook runs before/after actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub if_expr: Option<String>,
    #[serde(default)]
    pub async_exec: bool,
    #[serde(default)]
    pub timeout: Option<u64>,
}

// ============================================================================
// Tool
// ============================================================================

/// A custom tool provided by a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
}

// ============================================================================
// Plugin Loading
// ============================================================================

/// Loaded plugin with metadata
#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub enabled: bool,
    pub scope: PluginScope,
}

/// Plugin scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum PluginScope {
    #[default]
    User,
    Project,
    Local,
}


/// Plugin registry
#[derive(Debug, Clone, Default)]
pub struct PluginRegistry {
    pub plugins: HashMap<String, LoadedPlugin>,
    pub enabled_skills: Vec<Skill>,
    pub enabled_hooks: HookConfig,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load all plugins from plugin directories
    pub fn load_plugins(config: &Config) -> Result<Self> {
        let mut registry = Self::new();

        // Load user plugins
        let user_dir = Self::get_user_plugins_dir()?;
        if user_dir.exists() {
            Self::load_from_dir(&user_dir, PluginScope::User, config, &mut registry)?;
        }

        // Load project plugins
        if let Ok(cwd) = std::env::current_dir() {
            let project_dir = cwd.join(".claude").join("plugins");
            if project_dir.exists() {
                Self::load_from_dir(&project_dir, PluginScope::Project, config, &mut registry)?;
            }
        }

        // Aggregate skills and hooks
        for plugin in registry.plugins.values() {
            if plugin.enabled {
                registry.enabled_skills.extend(plugin.manifest.skills.clone());
                Self::merge_hooks(&mut registry.enabled_hooks, &plugin.manifest.hooks);
            }
        }

        Ok(registry)
    }

    fn load_from_dir(
        dir: &PathBuf,
        scope: PluginScope,
        config: &Config,
        registry: &mut PluginRegistry,
    ) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Check for .claude-plugin directory or regular plugin directory
            let plugin_dir = if path.is_dir() && path.file_name().and_then(|n| n.to_str()) == Some(".claude-plugin") {
                path.clone()
            } else if path.is_dir() {
                let manifest = path.join("plugin.json");
                let toml = path.join("plugin.toml");
                let claude_plugin = path.join(".claude-plugin");
                if manifest.exists() || toml.exists() || claude_plugin.exists() {
                    path.clone()
                } else {
                    continue;
                }
            } else {
                continue;
            };

            if let Ok(manifest) = Self::load_plugin(&plugin_dir, scope) {
                let enabled = Self::is_plugin_enabled(config, &manifest.name);
                let loaded = LoadedPlugin {
                    manifest,
                    path: plugin_dir,
                    enabled,
                    scope,
                };
                registry.plugins.insert(loaded.manifest.name.clone(), loaded);
            }
        }

        Ok(())
    }

    fn load_plugin(path: &Path, _scope: PluginScope) -> Result<PluginManifest> {
        // Try .claude-plugin/plugin.json first
        let manifest_path = path.join(".claude-plugin").join("plugin.json");
        if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path)?;
            let manifest: PluginManifest = serde_json::from_str(&content)
                .context("Failed to parse plugin.json")?;
            return Ok(manifest);
        }

        // Try plugin.json in root
        let manifest_path = path.join("plugin.json");
        if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path)?;
            let manifest: PluginManifest = serde_json::from_str(&content)
                .context("Failed to parse plugin.json")?;
            return Ok(manifest);
        }

        // Try plugin.toml
        let toml_path = path.join("plugin.toml");
        if toml_path.exists() {
            let content = fs::read_to_string(&toml_path)?;
            let manifest: PluginManifest = serde_yaml::from_str(&content)
                .context("Failed to parse plugin.toml")?;
            return Ok(manifest);
        }

        anyhow::bail!("No plugin.json, .claude-plugin/plugin.json, or plugin.toml found");
    }

    fn merge_hooks(target: &mut HookConfig, source: &HookConfig) {
        target.pre_tool_use.extend(source.pre_tool_use.clone());
        target.post_tool_use.extend(source.post_tool_use.clone());
        target.post_tool_use_failure.extend(source.post_tool_use_failure.clone());
        target.permission_denied.extend(source.permission_denied.clone());
        target.permission_request.extend(source.permission_request.clone());
        target.user_prompt_submit.extend(source.user_prompt_submit.clone());
        target.session_start.extend(source.session_start.clone());
        target.session_end.extend(source.session_end.clone());
        target.stop.extend(source.stop.clone());
        target.pre_compact.extend(source.pre_compact.clone());
        target.post_compact.extend(source.post_compact.clone());
        target.file_changed.extend(source.file_changed.clone());
        target.cwd_changed.extend(source.cwd_changed.clone());
        target.setup.extend(source.setup.clone());
        target.task_created.extend(source.task_created.clone());
        target.task_completed.extend(source.task_completed.clone());
    }

    fn is_plugin_enabled(config: &Config, name: &str) -> bool {
        config.project_choices
            .get("plugins")
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.get(name))
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }

    fn get_user_plugins_dir() -> Result<PathBuf> {
        let base = dirs::config_dir()
            .context("Could not find config directory")?
            .join("code-buddy")
            .join("plugins");

        if !base.exists() {
            fs::create_dir_all(&base)?;
        }

        Ok(base)
    }

    /// Get the skills directory for user plugins
    pub fn get_skills_dir(_config: &Config) -> Result<PathBuf> {
        let base = dirs::config_dir()
            .context("Could not find config directory")?
            .join("code-buddy")
            .join("skills");

        if !base.exists() {
            fs::create_dir_all(&base)?;
        }

        Ok(base)
    }

    /// Load skills from the skills directory
    pub fn load_skills(config: &Config) -> Result<Vec<Skill>> {
        let skills_dir = Self::get_skills_dir(config)?;
        let mut skills = Vec::new();

        if !skills_dir.exists() {
            return Ok(skills);
        }

        for entry in fs::read_dir(&skills_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let skill_file = path.join("SKILL.md");
                let alt_file = path.join("skill.md");
                let txt_file = path.join("skill.txt");

                let file = if skill_file.exists() {
                    skill_file
                } else if alt_file.exists() {
                    alt_file
                } else if txt_file.exists() {
                    txt_file
                } else {
                    continue;
                };

                let content = fs::read_to_string(&file)?;
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let skill = Self::parse_skill_file(&name, &content);
                skills.push(skill);
            }
        }

        Ok(skills)
    }

    fn parse_skill_file(name: &str, content: &str) -> Skill {
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Skill {
                name: name.to_string(),
                description: "No description".to_string(),
                ..Default::default()
            };
        }

        // Check for YAML frontmatter
        if lines.first() == Some(&"---") {
            let mut frontmatter_end = 0;
            for (i, line) in lines.iter().enumerate().skip(1) {
                if *line == "---" {
                    frontmatter_end = i;
                    break;
                }
            }

            if frontmatter_end > 1 {
                let frontmatter_lines = &lines[1..frontmatter_end];

                let description = frontmatter_lines.iter()
                    .find(|l| l.starts_with("description:"))
                    .map(|l| l.trim_start_matches("description:").trim())
                    .unwrap_or("No description")
                    .to_string();

                let when_to_use = frontmatter_lines.iter()
                    .find(|l| l.starts_with("when_to_use:"))
                    .map(|l| l.trim_start_matches("when_to_use:").trim().to_string());

                let allowed_tools: Vec<String> = frontmatter_lines.iter()
                    .find(|l| l.starts_with("allowed-tools:"))
                    .map(|l| {
                        let val = l.trim_start_matches("allowed-tools:");
                        serde_yaml::from_str(val).unwrap_or_default()
                    })
                    .unwrap_or_default();

                let argument_hint = frontmatter_lines.iter()
                    .find(|l| l.starts_with("argument-hint:"))
                    .map(|l| l.trim_start_matches("argument-hint:").trim().to_string());

                let user_invocable = frontmatter_lines.iter()
                    .find(|l| l.starts_with("user-invocable:"))
                    .map(|l| l.trim_start_matches("user-invocable:").trim() == "true")
                    .unwrap_or(true);

                let paths: Vec<String> = frontmatter_lines.iter()
                    .find(|l| l.starts_with("paths:"))
                    .map(|l| {
                        let val = l.trim_start_matches("paths:");
                        serde_yaml::from_str(val).unwrap_or_default()
                    })
                    .unwrap_or_default();

                let _prompt = lines[frontmatter_end + 1..].join("\n");

                return Skill {
                    name: name.to_string(),
                    description,
                    when_to_use,
                    allowed_tools,
                    argument_hint,
                    user_invocable,
                    paths,
                    ..Default::default()
                };
            }
        }

        // No frontmatter - use first line as description
        Skill {
            name: name.to_string(),
            description: lines[0].to_string(),
            user_invocable: true,
            ..Default::default()
        }
    }

    /// Get all available skills (from plugins + skills directory)
    pub fn get_all_skills(&self, config: &Config) -> Result<Vec<Skill>> {
        let mut skills = self.enabled_skills.clone();
        skills.extend(Self::load_skills(config)?);
        Ok(skills)
    }

    /// Install a plugin from a directory or URL
    pub fn install_plugin(_config: &Config, source: &str, scope: PluginScope) -> Result<String> {
        let plugins_dir = Self::get_plugins_dir_for_scope(scope)?;
        let name = Self::extract_plugin_name(source);

        // SECURITY: Validate the extracted name to prevent path traversal attacks.
        // A malicious URL like "https://example.com/../../../etc" could extract to ".."
        // which would allow writing outside the plugins directory.
        if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
            anyhow::bail!(
                "Invalid plugin name '{}': must not be empty or contain path traversal sequences",
                name
            );
        }

        // Canonicalize both paths to ensure the destination stays within the plugins directory.
        // This handles cases where relative path components could escape via symlinks or unusual paths.
        let dest_canonical = plugins_dir.join(&name).canonicalize()
            .unwrap_or_else(|_| plugins_dir.join(&name));
        let plugins_dir_canonical = plugins_dir.canonicalize()
            .unwrap_or_else(|_| plugins_dir.clone());
        if !dest_canonical.to_string_lossy().starts_with(&format!("{}{}", plugins_dir_canonical.to_string_lossy(), std::path::MAIN_SEPARATOR)) {
            anyhow::bail!(
                "Plugin installation would escape the plugins directory: '{}' resolves to '{}'",
                name,
                dest_canonical.display()
            );
        }

        let dest = plugins_dir.join(&name);

        if dest.exists() {
            anyhow::bail!("Plugin '{}' is already installed", name);
        }

        if source.starts_with("http") || source.starts_with("git@") || source.contains("github.com") {
            // Clone from git URL
            Self::install_from_git(source, &dest)?;
        } else {
            // Copy from local directory
            let src = PathBuf::from(source);
            if !src.exists() {
                anyhow::bail!("Source path does not exist: {}", source);
            }
            fs::create_dir_all(&dest)?;
            Self::copy_dir(&src, &dest)?;
        }

        Ok(name)
    }

    fn get_plugins_dir_for_scope(scope: PluginScope) -> Result<PathBuf> {
        let base = match scope {
            PluginScope::User => dirs::config_dir()
                .context("Could not find config directory")?
                .join("code-buddy")
                .join("plugins"),
            PluginScope::Project => {
                let cwd = std::env::current_dir()
                    .context("Could not get current directory")?;
                cwd.join(".claude").join("plugins")
            }
            PluginScope::Local => {
                // Local plugins are not persisted
                anyhow::bail!("Local scope plugins cannot be installed");
            }
        };

        if !base.exists() {
            fs::create_dir_all(&base)?;
        }

        Ok(base)
    }

    fn install_from_git(url: &str, dest: &PathBuf) -> Result<()> {
        // SECURITY: Validate URL format to prevent command injection
        // Reject URLs containing shell metacharacters or suspicious patterns
        let url_lower = url.to_lowercase();
        let suspicious_patterns = ["&&", "||", ";", "|", "`", "$(", "$("];
        for pattern in suspicious_patterns {
            if url.contains(pattern) {
                anyhow::bail!(
                    "URL contains suspicious pattern '{}': command injection attempt blocked",
                    pattern
                );
            }
        }

        // Validate URL format: must be http(s):// or git@ (SSH)
        if !url_lower.starts_with("https://")
            && !url_lower.starts_with("http://")
            && !url_lower.starts_with("git@")
        {
            anyhow::bail!(
                "URL must use https://, http://, or git@ protocol. Got: {}",
                url
            );
        }

        let output = std::process::Command::new("git")
            .args(["clone", "--depth", "1", "--", url])
            .arg(dest)
            .output()
            .context("Failed to clone repository")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git clone failed: {}", stderr);
        }

        Ok(())
    }

    fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = match path.file_name() {
                Some(name) => dest.join(name),
                None => continue,
            };

            if path.is_dir() {
                fs::create_dir_all(&dest_path)?;
                Self::copy_dir(path.as_ref(), &dest_path)?;
            } else {
                fs::copy(path, &dest_path)?;
            }
        }
        Ok(())
    }

    fn extract_plugin_name(source: &str) -> String {
        // Extract name from URL or path
        source
            .trim_end_matches('/')
            .split('/')
            .next_back()
            .unwrap_or("plugin")
            .trim_end_matches(".git")
            .to_string()
    }

    /// Uninstall a plugin
    pub fn uninstall_plugin(_config: &Config, name: &str) -> Result<()> {
        // Try user plugins first
        if let Ok(user_dir) = Self::get_user_plugins_dir() {
            let user_plugin = user_dir.join(name);
            if user_plugin.exists() {
                fs::remove_dir_all(&user_plugin)
                    .context("Failed to remove plugin directory")?;
                return Ok(());
            }
        }

        // Try project plugins
        let cwd = std::env::current_dir()
            .context("Could not get current directory")?;
        let project_plugin = cwd.join(".claude").join("plugins").join(name);
        if project_plugin.exists() {
            fs::remove_dir_all(&project_plugin)
                .context("Failed to remove plugin directory")?;
            return Ok(());
        }

        anyhow::bail!("Plugin '{}' is not installed", name);
    }

    /// Enable a plugin
    pub fn enable_plugin(config: &mut Config, name: &str, enabled: bool) -> Result<()> {
        // Use serde_json::Value for nested table operations
        use serde_json::Value;

        let plugins = config.project_choices
            .entry("plugins".to_string())
            .or_insert_with(|| Value::Object(Default::default()));

        let plugins_obj = plugins.as_object_mut()
            .context("Expected plugins to be an object")?;

        let enabled_table = plugins_obj
            .entry("enabled".to_string())
            .or_insert_with(|| Value::Object(Default::default()));

        let enabled_map = enabled_table
            .as_object_mut()
            .context("Expected enabled to be an object")?;

        enabled_map.insert(name.to_string(), Value::Bool(enabled));

        config.save()?;
        Ok(())
    }

    /// Validate a plugin at a given path
    pub fn validate_plugin(path: &Path) -> Result<Vec<PluginValidationError>> {
        let mut errors = Vec::new();

        // Check for manifest
        let manifest_path = path.join("plugin.json");
        let toml_path = path.join("plugin.toml");
        let claude_plugin = path.join(".claude-plugin").join("plugin.json");

        if !manifest_path.exists() && !toml_path.exists() && !claude_plugin.exists() {
            errors.push(PluginValidationError {
                code: "manifest-not-found".to_string(),
                message: "No plugin.json, plugin.toml, or .claude-plugin/plugin.json found".to_string(),
            });
            return Ok(errors); // No point continuing without manifest
        }

        // Load manifest to validate
        let manifest = match Self::load_plugin(path, PluginScope::Local) {
            Ok(m) => m,
            Err(e) => {
                errors.push(PluginValidationError {
                    code: "manifest-parse-error".to_string(),
                    message: format!("Failed to parse manifest: {}", e),
                });
                return Ok(errors);
            }
        };

        // Validate name
        if manifest.name.is_empty() {
            errors.push(PluginValidationError {
                code: "manifest-validation-error".to_string(),
                message: "Plugin name is required".to_string(),
            });
        }

        // Validate version
        if manifest.version.is_empty() {
            errors.push(PluginValidationError {
                code: "manifest-validation-error".to_string(),
                message: "Plugin version is required".to_string(),
            });
        }

        // Validate hooks
        for hook in manifest.hooks.pre_tool_use.iter()
            .chain(manifest.hooks.post_tool_use.iter())
            .chain(manifest.hooks.session_start.iter()) {
            if hook.command.is_none() && hook.prompt.is_none() && hook.url.is_none() {
                errors.push(PluginValidationError {
                    code: "manifest-validation-error".to_string(),
                    message: "Hook must have command, prompt, or url defined".to_string(),
                });
            }
        }

        Ok(errors)
    }

    /// List available plugins from marketplace
    pub fn list_marketplace_plugins(_marketplace: &str) -> Result<Vec<MarketplacePlugin>> {
        // Placeholder for marketplace listing
        // In real implementation, would fetch from GitHub or configured marketplace
        Ok(vec![])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginValidationError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub downloads: u64,
}

// ============================================================================
// Built-in Plugins
// ============================================================================

/// Built-in plugin definition
pub struct BuiltinPlugin {
    pub name: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    pub skills: Vec<Skill>,
    pub hooks: HookConfig,
}

/// Get all built-in plugins
pub fn get_builtin_plugins() -> Vec<BuiltinPlugin> {
    vec![
        BuiltinPlugin {
            name: "builtin-default",
            description: "Default skills and hooks for Code Buddy",
            version: env!("CARGO_PKG_VERSION"),
            skills: vec![
                Skill {
                    name: "help".to_string(),
                    description: "Get help with Code Buddy".to_string(),
                    when_to_use: Some("When you need help understanding how to use Code Buddy".to_string()),
                    user_invocable: true,
                    ..Default::default()
                },
                Skill {
                    name: "debug".to_string(),
                    description: "Enable debug mode".to_string(),
                    when_to_use: Some("When you need to troubleshoot issues".to_string()),
                    user_invocable: true,
                    ..Default::default()
                },
            ],
            hooks: HookConfig {
                session_start: vec![
                    HookDef {
                        hook_type: HookType::Command,
                        command: Some("echo 'Code Buddy initialized'".to_string()),
                        ..Default::default()
                    }
                ],
                ..Default::default()
            },
        },
    ]
}

impl Default for Skill {
    fn default() -> Self {
        Skill {
            name: String::new(),
            description: String::new(),
            when_to_use: None,
            allowed_tools: vec![],
            argument_hint: None,
            arguments: vec![],
            model: None,
            user_invocable: true,
            context: None,
            agent: None,
            effort: None,
            paths: vec![],
        }
    }
}


impl Default for HookDef {
    fn default() -> Self {
        HookDef {
            hook_type: HookType::Command,
            command: None,
            prompt: None,
            matcher: None,
            if_cond: None,
            timeout: None,
            async_exec: false,
            url: None,
            headers: None,
            allowed_env_vars: vec![],
            model: None,
        }
    }
}