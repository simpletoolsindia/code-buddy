//! Skin/Theme Engine - Data-driven CLI visual customization
//!
//! Provides data-driven CLI theming. Skins are pure data - no code changes needed.
//! 4 built-in skins: default, ares, mono, slate
//! User skins in: ~/.code-buddy/skins/*.yaml

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Skin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinConfig {
    pub name: String,
    pub description: String,
    pub colors: SkinColors,
    pub spinner: SkinSpinner,
    pub branding: SkinBranding,
    pub tool_prefix: String,
    pub tool_emojis: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinColors {
    pub banner_border: String,
    pub banner_title: String,
    pub banner_accent: String,
    pub banner_dim: String,
    pub banner_text: String,
    pub response_border: String,
    pub response_label: String,
    pub error: String,
    pub warning: String,
    pub success: String,
    pub info: String,
    pub prompt_symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinSpinner {
    pub waiting_faces: Vec<String>,
    pub thinking_faces: Vec<String>,
    pub thinking_verbs: Vec<String>,
    pub wings: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinBranding {
    pub agent_name: String,
    pub welcome: String,
    pub response_label: String,
    pub prompt_symbol: String,
}

impl Default for SkinConfig {
    fn default() -> Self {
        built_in_skins().get("default").cloned().unwrap()
    }
}

/// Built-in skins
pub fn built_in_skins() -> HashMap<String, SkinConfig> {
    let mut skins = HashMap::new();

    // Default skin - Classic gold/kawaii
    skins.insert("default".to_string(), SkinConfig {
        name: "default".to_string(),
        description: "Classic Hermes gold/kawaii theme".to_string(),
        colors: SkinColors {
            banner_border: "#FFD700".to_string(),
            banner_title: "#FFD700".to_string(),
            banner_accent: "#FFA500".to_string(),
            banner_dim: "#888888".to_string(),
            banner_text: "#FFFFFF".to_string(),
            response_border: "#FFD700".to_string(),
            response_label: "#FFD700".to_string(),
            error: "#FF4444".to_string(),
            warning: "#FFA500".to_string(),
            success: "#44FF44".to_string(),
            info: "#44AAFF".to_string(),
            prompt_symbol: "☤".to_string(),
        },
        spinner: SkinSpinner {
            waiting_faces: vec!["(・ω・)".to_string(), "(・´∇・)".to_string(), "(´・ω・`)".to_string()],
            thinking_faces: vec!["(｀・ω・´)".to_string(), "( ´ω`)".to_string(), "(´・∀・`)".to_string()],
            thinking_verbs: vec![
                "thinking".to_string(), "processing".to_string(), "analyzing".to_string(),
                "consulting".to_string(), "researching".to_string(), "reasoning".to_string(),
            ],
            wings: Some(vec![vec!["⟨✧".to_string(), "✧⟩".to_string()]]),
        },
        branding: SkinBranding {
            agent_name: "Code Buddy".to_string(),
            welcome: "Hello! I'm Code Buddy, your AI coding assistant.".to_string(),
            response_label: " Code Buddy ".to_string(),
            prompt_symbol: "☤".to_string(),
        },
        tool_prefix: "⚡".to_string(),
        tool_emojis: HashMap::new(),
    });

    // Ares skin - Crimson/bronze war-god
    skins.insert("ares".to_string(), SkinConfig {
        name: "ares".to_string(),
        description: "Crimson/bronze war-god theme with custom spinner wings".to_string(),
        colors: SkinColors {
            banner_border: "#8B0000".to_string(),
            banner_title: "#CD5C5C".to_string(),
            banner_accent: "#B8860B".to_string(),
            banner_dim: "#666666".to_string(),
            banner_text: "#F5F5DC".to_string(),
            response_border: "#8B0000".to_string(),
            response_label: "#CD5C5C".to_string(),
            error: "#FF6347".to_string(),
            warning: "#DAA520".to_string(),
            success: "#9ACD32".to_string(),
            info: "#87CEEB".to_string(),
            prompt_symbol: "⛨".to_string(),
        },
        spinner: SkinSpinner {
            waiting_faces: vec!["[°◇°]".to_string(), "[◇°◇]".to_string()],
            thinking_faces: vec!["[°▽°]".to_string(), "[◇°ω°]".to_string()],
            thinking_verbs: vec![
                "strategizing".to_string(), "forging".to_string(), "preparing".to_string(),
            ],
            wings: Some(vec![
                vec!["⟨⚔".to_string(), "⚔⟩".to_string()],
                vec!["⟨☠".to_string(), "☠⟩".to_string()],
            ]),
        },
        branding: SkinBranding {
            agent_name: "ARES".to_string(),
            welcome: "I am ARES. State your purpose.".to_string(),
            response_label: " ARES ".to_string(),
            prompt_symbol: "⛨".to_string(),
        },
        tool_prefix: "⚔".to_string(),
        tool_emojis: HashMap::new(),
    });

    // Mono skin - Clean grayscale
    skins.insert("mono".to_string(), SkinConfig {
        name: "mono".to_string(),
        description: "Clean grayscale monochrome theme".to_string(),
        colors: SkinColors {
            banner_border: "#666666".to_string(),
            banner_title: "#FFFFFF".to_string(),
            banner_accent: "#AAAAAA".to_string(),
            banner_dim: "#555555".to_string(),
            banner_text: "#CCCCCC".to_string(),
            response_border: "#444444".to_string(),
            response_label: "#888888".to_string(),
            error: "#FF6666".to_string(),
            warning: "#FFCC00".to_string(),
            success: "#66FF66".to_string(),
            info: "#66CCFF".to_string(),
            prompt_symbol: "$".to_string(),
        },
        spinner: SkinSpinner {
            waiting_faces: vec!["[   ]".to_string(), "[.  ]".to_string(), "[.. ]".to_string(), "[...]".to_string()],
            thinking_faces: vec!["[*  ]".to_string(), "[** ]".to_string(), "[***]".to_string()],
            thinking_verbs: vec![
                "processing".to_string(), "loading".to_string(), "ready".to_string(),
            ],
            wings: None,
        },
        branding: SkinBranding {
            agent_name: "code-buddy".to_string(),
            welcome: "code-buddy v2.1.89".to_string(),
            response_label: " output ".to_string(),
            prompt_symbol: "$".to_string(),
        },
        tool_prefix: ">".to_string(),
        tool_emojis: HashMap::new(),
    });

    // Slate skin - Cool blue developer theme
    skins.insert("slate".to_string(), SkinConfig {
        name: "slate".to_string(),
        description: "Cool blue developer-focused theme".to_string(),
        colors: SkinColors {
            banner_border: "#1E3A5F".to_string(),
            banner_title: "#5DADE2".to_string(),
            banner_accent: "#3498DB".to_string(),
            banner_dim: "#566573".to_string(),
            banner_text: "#EAECEE".to_string(),
            response_border: "#1E3A5F".to_string(),
            response_label: "#85C1E9".to_string(),
            error: "#E74C3C".to_string(),
            warning: "#F39C12".to_string(),
            success: "#2ECC71".to_string(),
            info: "#3498DB".to_string(),
            prompt_symbol: ">".to_string(),
        },
        spinner: SkinSpinner {
            waiting_faces: vec!["[○   ]".to_string(), "[ ○  ]".to_string(), "[  ○ ]".to_string(), "[   ○]".to_string()],
            thinking_faces: vec!["[●   ]".to_string(), "[ ●  ]".to_string(), "[  ● ]".to_string()],
            thinking_verbs: vec![
                "compiling".to_string(), "debugging".to_string(), "deploying".to_string(),
                "linting".to_string(), "testing".to_string(), "building".to_string(),
            ],
            wings: Some(vec![vec!["[".to_string(), "]".to_string()]]),
        },
        branding: SkinBranding {
            agent_name: "code-buddy".to_string(),
            welcome: "Ready to code.".to_string(),
            response_label: " output ".to_string(),
            prompt_symbol: ">".to_string(),
        },
        tool_prefix: "│".to_string(),
        tool_emojis: HashMap::new(),
    });

    skins
}

/// Load a skin by name
pub fn load_skin(name: &str, custom_skins_dir: Option<PathBuf>) -> SkinConfig {
    // Try user skins first
    if let Some(dir) = custom_skins_dir {
        let skin_path = dir.join(format!("{}.yaml", name));
        if skin_path.exists() {
            if let Ok(content) = fs::read_to_string(&skin_path) {
                if let Ok(skin) = serde_yaml::from_str::<SkinConfig>(&content) {
                    return skin;
                }
            }
        }
    }

    // Try built-in skins
    if let Some(skin) = built_in_skins().get(name) {
        return skin.clone();
    }

    // Fall back to default
    SkinConfig::default()
}

/// List available skins
pub fn list_skins(custom_skins_dir: Option<PathBuf>) -> String {
    let mut md = String::from("# Available Skins\n\n");

    md.push_str("## Built-in Skins\n\n");
    for (name, skin) in built_in_skins() {
        md.push_str(&format!("- **{}** - {}\n", name, skin.description));
    }

    // List user skins
    if let Some(dir) = custom_skins_dir {
        if dir.exists() {
            let user_skins: Vec<String> = fs::read_dir(&dir)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map(|ex| ex == "yaml").unwrap_or(false))
                        .filter_map(|e| e.path().file_stem().and_then(|s| s.to_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            if !user_skins.is_empty() {
                md.push_str("\n## User Skins\n\n");
                for name in user_skins {
                    md.push_str(&format!("- **{}** (user-installed)\n", name));
                }
            }
        }
    }

    md.push_str("\nUse `/skin <name>` to switch skins.\n");

    md
}

/// Apply ANSI color codes from hex
pub fn hex_to_ansi(hex: &str) -> String {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return String::new();
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);

    format!("\x1b[38;2;{};{};{}m", r, g, b)
}

/// Reset ANSI color
pub fn ansi_reset() -> String {
    "\x1b[0m".to_string()
}

/// Format text with skin colors
pub fn format_with_skin(skin: &SkinConfig, text: &str, color_type: &str) -> String {
    let color = match color_type {
        "banner_border" => &skin.colors.banner_border,
        "banner_title" => &skin.colors.banner_title,
        "banner_accent" => &skin.colors.banner_accent,
        "banner_dim" => &skin.colors.banner_dim,
        "banner_text" => &skin.colors.banner_text,
        "error" => &skin.colors.error,
        "warning" => &skin.colors.warning,
        "success" => &skin.colors.success,
        "info" => &skin.colors.info,
        _ => &skin.colors.banner_text,
    };

    format!("{}{}{}", hex_to_ansi(color), text, ansi_reset())
}

/// Format banner with skin
pub fn format_banner(skin: &SkinConfig, title: &str, lines: Vec<&str>) -> String {
    let width = 60;
    let border_char = "─";
    let top = format!("╔{}╗", border_char.repeat(width));
    let bottom = format!("╚{}╝", border_char.repeat(width));

    let mut result = format!("{}\n", hex_to_ansi(&skin.colors.banner_border));

    result.push_str(&format!("{}{}{}\n", hex_to_ansi(&skin.colors.banner_border), top, ansi_reset()));
    result.push_str(&format!(
        "{}{}{:^width$}{}\n",
        hex_to_ansi(&skin.colors.banner_border),
        "║",
        format_with_skin(skin, title, "banner_title"),
        ansi_reset()
    ));
    result.push_str(&format!("{}{}{}\n", hex_to_ansi(&skin.colors.banner_border), "╠".to_string() + &border_char.repeat(width) + "╣", ansi_reset()));

    for line in lines {
        result.push_str(&format!(
            "{}{}{:width$}{}\n",
            hex_to_ansi(&skin.colors.banner_border),
            "║",
            format_with_skin(skin, line, "banner_text"),
            ansi_reset()
        ));
    }

    result.push_str(&format!("{}{}{}\n", hex_to_ansi(&skin.colors.banner_border), bottom, ansi_reset()));

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default_skin() {
        let skin = load_skin("default", None);
        assert_eq!(skin.name, "default");
    }

    #[test]
    fn test_load_ares_skin() {
        let skin = load_skin("ares", None);
        assert_eq!(skin.name, "ares");
    }

    #[test]
    fn test_hex_to_ansi() {
        let result = hex_to_ansi("#FF0000");
        assert!(result.contains("255"));
    }

    #[test]
    fn test_skin_fallback() {
        let skin = load_skin("nonexistent", None);
        assert_eq!(skin.name, "default");
    }
}
