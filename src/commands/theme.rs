//! Theme Command - Terminal theme and color configuration
//!
//! Provides theme and color scheme management.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Available color themes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum Theme {
    Dark,
    Light,
    #[default]
    Auto,
    Monokai,
    Dracula,
    Nord,
    Solarized,
    OneDark,
}

impl Theme {
    pub fn all() -> Vec<Self> {
        vec![
            Theme::Dark,
            Theme::Light,
            Theme::Auto,
            Theme::Monokai,
            Theme::Dracula,
            Theme::Nord,
            Theme::Solarized,
            Theme::OneDark,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Auto => "Auto (system)",
            Theme::Monokai => "Monokai",
            Theme::Dracula => "Dracula",
            Theme::Nord => "Nord",
            Theme::Solarized => "Solarized",
            Theme::OneDark => "One Dark",
        }
    }

    pub fn colors(&self) -> ThemeColors {
        match self {
            Theme::Dark => ThemeColors {
                background: "#1e1e1e",
                foreground: "#d4d4d4",
                accent: "#007acc",
                error: "#f14c4c",
                warning: "#cca700",
                success: "#4ec9b0",
                info: "#3794ff",
            },
            Theme::Light => ThemeColors {
                background: "#ffffff",
                foreground: "#333333",
                accent: "#0066cc",
                error: "#d32f2f",
                warning: "#f9a825",
                success: "#388e3c",
                info: "#1976d2",
            },
            Theme::Auto => ThemeColors::default(),
            Theme::Monokai => ThemeColors {
                background: "#272822",
                foreground: "#f8f8f2",
                accent: "#66d9ef",
                error: "#f92672",
                warning: "#fd971f",
                success: "#a6e22e",
                info: "#ae81ff",
            },
            Theme::Dracula => ThemeColors {
                background: "#282a36",
                foreground: "#f8f8f2",
                accent: "#bd93f9",
                error: "#ff5555",
                warning: "#f1fa8c",
                success: "#50fa7b",
                info: "#8be9fd",
            },
            Theme::Nord => ThemeColors {
                background: "#2e3440",
                foreground: "#d8dee9",
                accent: "#88c0d0",
                error: "#bf616a",
                warning: "#ebcb8b",
                success: "#a3be8c",
                info: "#81a1c1",
            },
            Theme::Solarized => ThemeColors {
                background: "#002b36",
                foreground: "#839496",
                accent: "#268bd2",
                error: "#dc322f",
                warning: "#b58900",
                success: "#859900",
                info: "#2aa198",
            },
            Theme::OneDark => ThemeColors {
                background: "#282c34",
                foreground: "#abb2bf",
                accent: "#61afef",
                error: "#e06c75",
                warning: "#e5c07b",
                success: "#98c379",
                info: "#56b6c2",
            },
        }
    }
}


/// Theme colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub background: &'static str,
    pub foreground: &'static str,
    pub accent: &'static str,
    pub error: &'static str,
    pub warning: &'static str,
    pub success: &'static str,
    pub info: &'static str,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Theme::Dark.colors()
    }
}

/// Run theme command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return list_themes();
    }

    match args[0].as_str() {
        "list" | "ls" => list_themes(),
        "get" | "current" => current_theme(),
        "set" => {
            if args.len() < 2 {
                return Ok("Usage: theme set <theme-name>".to_string());
            }
            set_theme(&args[1])
        }
        "preview" => {
            if args.len() < 2 {
                return Ok("Usage: theme preview <theme-name>".to_string());
            }
            preview_theme(&args[1])
        }
        _ => {
            Ok(format!("Unknown theme command: {}\n\nUsage: theme <list|get|set|preview>", args[0]))
        }
    }
}

fn list_themes() -> Result<String> {
    let mut output = String::from("# Available Themes\n\n");
    for theme in Theme::all() {
        output.push_str(&format!("- {} - {}\n", theme.name(), theme.name()));
    }
    output.push('\n');
    output.push_str("Use `theme set <name>` to change the theme.\n");
    Ok(output)
}

fn current_theme() -> Result<String> {
    Ok(String::from("# Current Theme\n\nTheme: Auto (system)\n\nColors:\n- Background: #282a36\n- Foreground: #f8f8f2\n- Accent: #bd93f9\n"))
}

fn set_theme(name: &str) -> Result<String> {
    let name_lower = name.to_lowercase();
    let theme = Theme::all()
        .into_iter()
        .find(|t| t.name().to_lowercase() == name_lower);

    if let Some(theme) = theme {
        Ok(format!(
            "Theme set to: {}\n\nRun `theme preview {}` to see the colors.",
            theme.name(),
            theme.name()
        ))
    } else {
        Ok(format!(
            "Unknown theme: {}\n\nAvailable themes: {}",
            name,
            Theme::all()
                .iter()
                .map(|t| t.name())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

fn preview_theme(name: &str) -> Result<String> {
    let name_lower = name.to_lowercase();
    let theme = Theme::all()
        .into_iter()
        .find(|t| t.name().to_lowercase() == name_lower);

    if let Some(theme) = theme {
        let colors = theme.colors();
        Ok(format!(
            "# {} Theme Preview\n\n\
            Background: {}\n\
            Foreground: {}\n\
            Accent:     {}\n\
            Error:      {}\n\
            Warning:    {}\n\
            Success:    {}\n\
            Info:       {}\n",
            theme.name(),
            colors.background,
            colors.foreground,
            colors.accent,
            colors.error,
            colors.warning,
            colors.success,
            colors.info
        ))
    } else {
        Ok(format!("Unknown theme: {}", name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_list() {
        let themes = Theme::all();
        assert!(!themes.is_empty());
        assert!(themes.contains(&Theme::Dark));
    }

    #[test]
    fn test_theme_colors() {
        let colors = Theme::Dracula.colors();
        assert_eq!(colors.background, "#282a36");
        assert_eq!(colors.foreground, "#f8f8f2");
    }
}
