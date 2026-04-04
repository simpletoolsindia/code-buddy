//! First-run setup and theme selection

use crate::state::AppState;
use anyhow::Result;
use std::io::{self, Write};

/// Run first-run setup if needed
pub async fn run(state: &mut AppState) -> Result<()> {
    if !state.config.first_run {
        return Ok(());
    }

    print_welcome();
    let theme = ask_theme_preference()?;

    // Save the theme preference
    state.config.theme = Some(theme.clone());
    state.config.first_run = false;
    state.save_config()?;

    print_theme_selected(&theme);

    Ok(())
}

/// Print welcome message
fn print_welcome() {
    println!();
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                                                                ║");
    println!("║     ██████╗ ███████╗██╗   ██╗███╗   ███╗                      ║");
    println!("║     ██╔══██╗██╔════╝██║   ██║████╗ ████║                      ║");
    println!("║     ██║  ██║█████╗  ██║   ██║██╔████╔██║                      ║");
    println!("║     ██║  ██║██╔══╝  ██║   ██║██║╚██╔╝██║                      ║");
    println!("║     ██████╔╝███████╗╚██████╔╝██║ ╚═╝ ██║                      ║");
    println!("║     ╚═════╝ ╚══════╝ ╚═════╝ ╚═╝     ╚═╝                      ║");
    println!("║                                                                ║");
    println!("║                    Your AI Coding Companion                      ║");
    println!("║                                                                ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Welcome to Code Buddy! Let's get you set up.");
    println!();
}

/// Ask user for theme preference
fn ask_theme_preference() -> Result<String> {
    println!("Choose your preferred theme:");
    println!();
    println!("  1) Dark  - Dark background with light text (default)");
    println!("  2) Light - Light background with dark text");
    println!("  3) Auto  - Follow system preference");
    println!();

    loop {
        print!("Enter choice (1/2/3) or theme name (dark/light/auto): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "1" | "dark" => return Ok("dark".to_string()),
            "2" | "light" => return Ok("light".to_string()),
            "3" | "auto" => return Ok("auto".to_string()),
            _ => {
                println!("Invalid choice. Please enter 1, 2, or 3.");
                println!();
            }
        }
    }
}

/// Print the selected theme
fn print_theme_selected(theme: &str) {
    println!();
    match theme {
        "dark" => {
            println!("✓ Theme set to Dark mode");
            println!("  Your terminal will use a dark color scheme.");
        }
        "light" => {
            println!("✓ Theme set to Light mode");
            println!("  Your terminal will use a light color scheme.");
        }
        "auto" => {
            println!("✓ Theme set to Auto mode");
            println!("  Theme will follow your system preference.");
        }
        _ => {}
    }
    println!();
    println!("You can change your theme anytime with: code-buddy /theme");
    println!();
}

/// Check if first run setup is needed
pub fn is_first_run(state: &AppState) -> bool {
    state.config.first_run
}
