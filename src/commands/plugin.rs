//! Plugin management commands

use crate::config::Config;
use crate::plugins::{PluginRegistry, PluginScope, HookConfig};
use crate::state::AppState;
use anyhow::Result;

/// Run plugin command
pub async fn run(subcommand: Option<PluginSubcommand>, state: &mut AppState) -> Result<i32> {
    match subcommand {
        Some(PluginSubcommand::List { json, all }) => list_plugins(state, json, all).await,
        Some(PluginSubcommand::Add { source, scope }) => add_plugin(&state.config, &source, scope.as_deref()).await,
        Some(PluginSubcommand::Remove { name }) => remove_plugin(&state.config, &name).await,
        Some(PluginSubcommand::Enable { name }) => enable_plugin(&state.config, &name, true).await,
        Some(PluginSubcommand::Disable { name }) => enable_plugin(&state.config, &name, false).await,
        Some(PluginSubcommand::Update { name }) => update_plugin(&state.config, name).await,
        Some(PluginSubcommand::Search { query }) => search_plugins(&query).await,
        Some(PluginSubcommand::Skills) => list_skills(state).await,
        Some(PluginSubcommand::Marketplace { subcmd }) => marketplace(&state.config, subcmd).await,
        Some(PluginSubcommand::Validate { path }) => validate_plugin(&path).await,
        Some(PluginSubcommand::Reload) => reload_plugins(state).await,
        None => list_plugins(state, false, false).await,
    }
}

async fn list_plugins(state: &AppState, json: bool, all: bool) -> Result<i32> {
    let registry = PluginRegistry::load_plugins(&state.config)?;

    if json {
        println!("{{");
        println!("  \"plugins\": [");
        for (i, (name, plugin)) in registry.plugins.iter().enumerate() {
            let comma = if i < registry.plugins.len() - 1 { "," } else { "" };
            println!("    {{");
            println!("      \"name\": \"{}\",", name);
            println!("      \"version\": \"{}\",", plugin.manifest.version);
            println!("      \"description\": \"{}\",", plugin.manifest.description);
            println!("      \"enabled\": {},", plugin.enabled);
            println!("      \"scope\": \"{:?}\"", plugin.scope);
            println!("    }}{}", comma);
        }
        println!("  ],");
        println!("  \"skills_count\": {}", registry.enabled_skills.len());
        println!("}}");
    } else {
        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                    Plugin Registry                            ║");
        println!("╠══════════════════════════════════════════════════════════════╣");

        if registry.plugins.is_empty() {
            println!("║  No plugins installed                                        ║");
            println!("║  Run 'code-buddy plugin add <source>' to install            ║");
        } else {
            for (name, plugin) in &registry.plugins {
                let status = if plugin.enabled { "✓" } else { "✗" };
                let scope_tag = match plugin.scope {
                    PluginScope::User => "[user]",
                    PluginScope::Project => "[proj]",
                    PluginScope::Local => "[local]",
                };
                println!("║  {} {} {:15} v{} {}", status, scope_tag, name, plugin.manifest.version, plugin.manifest.description);
                if all {
                    println!("║     Path: {}", plugin.path.display());
                    if !plugin.manifest.keywords.is_empty() {
                        println!("║     Tags: {}", plugin.manifest.keywords.join(", "));
                    }
                }
            }
        }

        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        let skills = registry.enabled_skills.len();
        let hooks = count_hooks(&registry.enabled_hooks);
        println!("  Plugins: {} | Enabled: {} | Skills: {} | Hooks: {}",
            registry.plugins.len(),
            registry.plugins.values().filter(|p| p.enabled).count(),
            skills,
            hooks
        );
        println!();
        println!("  Use 'code-buddy plugin skills' to see all skills");
    }

    Ok(0)
}

fn count_hooks(hooks: &HookConfig) -> usize {
    hooks.pre_tool_use.len()
        + hooks.post_tool_use.len()
        + hooks.post_tool_use_failure.len()
        + hooks.permission_denied.len()
        + hooks.permission_request.len()
        + hooks.user_prompt_submit.len()
        + hooks.session_start.len()
        + hooks.session_end.len()
        + hooks.stop.len()
        + hooks.pre_compact.len()
        + hooks.post_compact.len()
        + hooks.file_changed.len()
        + hooks.cwd_changed.len()
        + hooks.setup.len()
        + hooks.task_created.len()
        + hooks.task_completed.len()
}

async fn add_plugin(config: &Config, source: &str, scope: Option<&str>) -> Result<i32> {
    let scope = match scope.map(|s| s.to_lowercase()).as_deref() {
        Some("project") => PluginScope::Project,
        _ => PluginScope::User,
    };

    println!("Installing plugin from: {}", source);
    println!("Scope: {:?}", scope);

    match PluginRegistry::install_plugin(config, source, scope) {
        Ok(name) => {
            println!();
            println!("✓ Plugin '{}' installed successfully!", name);
            println!("  Run 'code-buddy plugin list' to see it");
            println!("  Run 'code-buddy plugin enable {}' to enable it", name);
            Ok(0)
        }
        Err(e) => {
            eprintln!();
            eprintln!("✗ Failed to install plugin: {}", e);
            Ok(1)
        }
    }
}

async fn remove_plugin(config: &Config, name: &str) -> Result<i32> {
    println!("Uninstalling plugin: {}", name);

    match PluginRegistry::uninstall_plugin(config, name) {
        Ok(()) => {
            println!();
            println!("✓ Plugin '{}' uninstalled", name);
            Ok(0)
        }
        Err(e) => {
            eprintln!("✗ Failed to uninstall plugin: {}", e);
            Ok(1)
        }
    }
}

async fn enable_plugin(config: &Config, name: &str, enabled: bool) -> Result<i32> {
    let mut cfg = config.clone();
    let action = if enabled { "enabled" } else { "disabled" };

    match PluginRegistry::enable_plugin(&mut cfg, name, enabled) {
        Ok(()) => {
            println!("✓ Plugin '{}' {}", name, action);
            Ok(0)
        }
        Err(e) => {
            eprintln!("✗ Failed to {} plugin: {}", action, e);
            Ok(1)
        }
    }
}

async fn update_plugin(_config: &Config, name: Option<String>) -> Result<i32> {
    if let Some(name) = name {
        println!("Updating plugin: {}", name);
    } else {
        println!("Updating all plugins...");
    }
    println!("Plugin update not yet implemented - use 'git pull' in plugin directory");
    Ok(0)
}

async fn search_plugins(query: &str) -> Result<i32> {
    println!("Searching plugins for: '{}'", query);
    println!();
    println!("Searching marketplace...");
    println!();
    println!("To install a plugin, run:");
    println!("  code-buddy plugin add <plugin-id>");
    println!("  code-buddy plugin add https://github.com/user/plugin-name");
    println!();
    Ok(0)
}

async fn marketplace(_config: &Config, subcmd: Option<MarketplaceAction>) -> Result<i32> {
    match subcmd {
        Some(MarketplaceAction::List) => {
            println!("Configured marketplaces:");
            println!("  - claude-code-plugins (default)");
            println!();
            println!("Run 'code-buddy plugin marketplace add <source>' to add a custom marketplace");
            Ok(0)
        }
        Some(MarketplaceAction::Add { source }) => {
            println!("Adding marketplace: {}", source);
            println!("Marketplace feature not yet implemented");
            Ok(1)
        }
        Some(MarketplaceAction::Remove { name }) => {
            println!("Removing marketplace: {}", name);
            println!("Marketplace feature not yet implemented");
            Ok(1)
        }
        Some(MarketplaceAction::Update { name }) => {
            println!("Updating marketplace: {:?}", name);
            println!("Marketplace feature not yet implemented");
            Ok(1)
        }
        None => {
            println!("Marketplace commands:");
            println!("  list    - List configured marketplaces");
            println!("  add     - Add a new marketplace");
            println!("  remove  - Remove a marketplace");
            println!("  update  - Update marketplace listings");
            Ok(0)
        }
    }
}

async fn validate_plugin(path: &str) -> Result<i32> {
    use std::path::PathBuf;

    let plugin_path = if PathBuf::from(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };

    println!("Validating plugin at: {}", plugin_path.display());

    match PluginRegistry::validate_plugin(&plugin_path) {
        Ok(errors) => {
            if errors.is_empty() {
                println!("✓ Plugin is valid");
                Ok(0)
            } else {
                eprintln!("✗ Plugin validation failed:");
                for error in errors {
                    eprintln!("  [{}] {}", error.code, error.message);
                }
                Ok(1)
            }
        }
        Err(e) => {
            eprintln!("✗ Validation error: {}", e);
            Ok(1)
        }
    }
}

async fn reload_plugins(state: &mut AppState) -> Result<i32> {
    println!("Reloading plugins...");

    match PluginRegistry::load_plugins(&state.config) {
        Ok(registry) => {
            state.plugin_registry = Some(registry);
            println!("✓ Plugins reloaded");
            Ok(0)
        }
        Err(e) => {
            eprintln!("✗ Failed to reload plugins: {}", e);
            Ok(1)
        }
    }
}

async fn list_skills(state: &AppState) -> Result<i32> {
    let registry = PluginRegistry::load_plugins(&state.config)?;
    let skills = registry.get_all_skills(&state.config)?;

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Available Skills                           ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    if skills.is_empty() {
        println!("║  No skills available                                         ║");
        println!("║                                                               ║");
        println!("║  Create skills in:                                            ║");
        println!("║    ~/.config/code-buddy/skills/<skill-name>/SKILL.md          ║");
    } else {
        for skill in &skills {
            let inv = if skill.user_invocable { "↗" } else { "•" };
            println!("║  /{} {:25} {}", inv, skill.name, skill.description);
            if let Some(hint) = &skill.argument_hint {
                println!("║     Arguments: {}", hint);
            }
            if let Some(when) = &skill.when_to_use {
                println!("║     Use when: {}", when);
            }
        }
    }

    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Total: {} skills", skills.len());
    println!();

    Ok(0)
}

// Plugin subcommands
#[derive(Debug, Clone)]
pub enum PluginSubcommand {
    List { json: bool, all: bool },
    Add { source: String, scope: Option<String> },
    Remove { name: String },
    Enable { name: String },
    Disable { name: String },
    Update { name: Option<String> },
    Search { query: String },
    Skills,
    Marketplace { subcmd: Option<MarketplaceAction> },
    Validate { path: String },
    Reload,
}

#[derive(Debug, Clone)]
pub enum MarketplaceAction {
    List,
    Add { source: String },
    Remove { name: String },
    Update { name: Option<String> },
}