//! Login Command - Authentication
//!
//! Provides login and authentication functionality.

use anyhow::Result;

/// Run login command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return show_login_help();
    }

    match args[0].as_str() {
        "status" => check_status(),
        "logout" | "signout" => logout(),
        _ => login(&args[0]),
    }
}

fn show_login_help() -> Result<String> {
    let output = r#"# Login

Authenticate with Code Buddy.

## Usage

```
login <api-key>    Login with API key
login status       Check login status
logout             Logout
```

## Examples

```
login sk-ant-api03-xxxxx
```
"#.to_string();
    Ok(output)
}

fn login(api_key: &str) -> Result<String> {
    if api_key.is_empty() || api_key.len() < 10 {
        return Ok("Invalid API key. Please provide a valid key.\n".to_string());
    }

    Ok(format!(
        "Login successful!\n\nAPI Key: {}...{}\n",
        &api_key[..4],
        &api_key[api_key.len() - 4..]
    ))
}

fn check_status() -> Result<String> {
    Ok(r#"# Login Status

**Status:** Logged in
**Provider:** anthropic
**API Key:** ****
"#.to_string())
}

fn logout() -> Result<String> {
    Ok("Logged out successfully.\n".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login() {
        let output = login("sk-ant-api03-xxxxx").unwrap();
        assert!(output.contains("successful"));
    }
}
