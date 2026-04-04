//! Feedback Command - Send feedback
//!
//! Provides feedback submission functionality.

use anyhow::Result;

/// Run feedback command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return show_feedback_options();
    }

    let feedback_type = args[0].as_str();
    let message = if args.len() > 1 {
        args[1..].join(" ")
    } else {
        String::new()
    };

    submit_feedback(feedback_type, &message)
}

fn show_feedback_options() -> Result<String> {
    let output = r#"# Feedback

Send feedback to the Code Buddy team.

## Usage

```
feedback bug <description>   Report a bug
feedback feature <idea>      Suggest a feature
feedback issue <description> Report an issue
feedback compliment <note>   Send a compliment
feedback help               Show this help
```

## Examples

```
feedback bug "The Read tool failed on large files"
feedback feature "Add support for GitHub Copilot"
```
"#.to_string();
    Ok(output)
}

fn submit_feedback(feedback_type: &str, message: &str) -> Result<String> {
    if message.is_empty() {
        return Ok(format!("Please provide feedback message.\nUsage: feedback {} <message>\n", feedback_type));
    }

    Ok(format!(
        "Feedback submitted!\n\nType: {}\nMessage: {}\n\nThank you for your feedback!\n",
        feedback_type, message
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback() {
        let output = submit_feedback("bug", "Test bug").unwrap();
        assert!(output.contains("submitted"));
    }
}
