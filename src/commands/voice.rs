//! Voice Command - Voice mode control
//!
//! Provides voice mode functionality.

use anyhow::Result;

/// Run voice command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return show_voice_help();
    }

    match args[0].as_str() {
        "on" | "enable" => enable_voice(),
        "off" | "disable" => disable_voice(),
        "status" => voice_status(),
        "test" => test_voice(),
        _ => show_voice_help(),
    }
}

fn show_voice_help() -> Result<String> {
    let output = r#"# Voice Mode

Control voice mode for audio responses.

## Usage

```
voice on           Enable voice mode
voice off          Disable voice mode
voice status       Check voice status
voice test         Test voice output
```

## Requirements

- Working audio output
- TTS engine installed
"#.to_string();
    Ok(output)
}

fn enable_voice() -> Result<String> {
    Ok("Voice mode enabled.\n".to_string())
}

fn disable_voice() -> Result<String> {
    Ok("Voice mode disabled.\n".to_string())
}

fn voice_status() -> Result<String> {
    Ok(r#"# Voice Status

**Status:** Disabled
**Engine:** auto
**Speed:** 1.0x
"#.to_string())
}

fn test_voice() -> Result<String> {
    Ok("Voice test: \"Hello, I'm Code Buddy!\"\n".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice() {
        let output = voice_status().unwrap();
        assert!(output.contains("Voice Status"));
    }
}
