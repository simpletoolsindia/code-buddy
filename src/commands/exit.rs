//! Exit Command - Exit Code Buddy
//!
//! Provides graceful exit functionality.

use anyhow::Result;

/// Exit code
#[derive(Debug, Clone)]
pub enum ExitCode {
    Success,
    Error,
    UserCancelled,
    Restart,
}

/// Run exit command
pub fn run(args: &[String]) -> Result<ExitCode> {
    let force = args.contains(&"--force".to_string()) || args.contains(&"-f".to_string());
    let restart = args.contains(&"--restart".to_string());

    if restart {
        println!("Restarting Code Buddy...\n");
        return Ok(ExitCode::Restart);
    }

    if !force {
        println!("Are you sure you want to exit? (y/N): ");
    }

    println!("Goodbye!\n");
    Ok(ExitCode::Success)
}

/// Format exit message
pub fn format_exit(code: &ExitCode) -> String {
    match code {
        ExitCode::Success => "Exited successfully.".to_string(),
        ExitCode::Error => "Exited with errors.".to_string(),
        ExitCode::UserCancelled => "Cancelled.".to_string(),
        ExitCode::Restart => "Restarting...".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code() {
        let code = ExitCode::Success;
        assert_eq!(format_exit(&code), "Exited successfully.");
    }
}
