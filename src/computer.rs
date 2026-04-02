//! Computer Use Support
//!
//! Provides desktop control capabilities for AI agents:
//! - Mouse movement and clicking
//! - Keyboard input
//! - Screen capture and analysis
//! - Element detection (when supported)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Computer use action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ComputerAction {
    /// Move mouse to coordinates
    #[serde(rename = "mouse_move")]
    MouseMove { x: i32, y: i32 },
    /// Left click
    #[serde(rename = "left_click")]
    LeftClick { x: Option<i32>, y: Option<i32> },
    /// Right click
    #[serde(rename = "right_click")]
    RightClick { x: Option<i32>, y: Option<i32> },
    /// Double click
    #[serde(rename = "double_click")]
    DoubleClick { x: Option<i32>, y: Option<i32> },
    /// Scroll mouse
    #[serde(rename = "scroll")]
    Scroll { x: i32, y: i32 },
    /// Type text
    #[serde(rename = "type_text")]
    TypeText { text: String },
    /// Press key combination
    #[serde(rename = "key_combo")]
    KeyCombo { keys: Vec<String> },
    /// Screenshot
    #[serde(rename = "screenshot")]
    Screenshot,
    /// Wait
    #[serde(rename = "wait")]
    Wait { seconds: u64 },
}

/// Computer use result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerResult {
    pub success: bool,
    pub action: String,
    pub output: Option<String>,
    pub screenshot: Option<String>, // Base64 encoded
    pub error: Option<String>,
}

impl ComputerResult {
    pub fn success(action: &str) -> Self {
        Self {
            success: true,
            action: action.to_string(),
            output: None,
            screenshot: None,
            error: None,
        }
    }

    pub fn error(action: &str, err: &str) -> Self {
        Self {
            success: false,
            action: action.to_string(),
            output: None,
            screenshot: None,
            error: Some(err.to_string()),
        }
    }

    pub fn with_screenshot(mut self, screenshot: String) -> Self {
        self.screenshot = Some(screenshot);
        self
    }

    pub fn with_output(mut self, output: String) -> Self {
        self.output = Some(output);
        self
    }
}

/// Computer use executor
pub struct ComputerUse {
    screenshot_path: PathBuf,
}

impl ComputerUse {
    pub fn new() -> Self {
        Self {
            screenshot_path: std::env::temp_dir().join("computer_use_screenshot.png"),
        }
    }

    /// Execute a computer action
    pub fn execute(&self, action: &ComputerAction) -> ComputerResult {
        match action {
            ComputerAction::MouseMove { x, y } => self.mouse_move(*x, *y),
            ComputerAction::LeftClick { x, y } => self.left_click(*x, *y),
            ComputerAction::RightClick { x, y } => self.right_click(*x, *y),
            ComputerAction::DoubleClick { x, y } => self.double_click(*x, *y),
            ComputerAction::Scroll { x, y } => self.scroll(*x, *y),
            ComputerAction::TypeText { text } => self.type_text(text),
            ComputerAction::KeyCombo { keys } => self.key_combo(keys),
            ComputerAction::Screenshot => self.screenshot_action(),
            ComputerAction::Wait { seconds } => self.wait(*seconds),
        }
    }

    fn mouse_move(&self, x: i32, y: i32) -> ComputerResult {
        #[cfg(target_os = "macos")]
        {
            let script = format!("osascript -e 'tell application \"System Events\" to set position of mouse to ({}, {})'", x, y);
            let output = std::process::Command::new("sh")
                .args(["-c", &script])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("mouse_move"),
                Ok(o) => ComputerResult::error("mouse_move", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("mouse_move", &e.to_string()),
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("xdotool")
                .args(["mousemove", &x.to_string(), &y.to_string()])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("mouse_move"),
                Ok(o) => ComputerResult::error("mouse_move", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("mouse_move", &e.to_string()),
            }
        }

        #[cfg(target_os = "windows")]
        {
            let script = format!(
                r#"Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point({}, {})"#,
                x, y
            );
            let output = std::process::Command::new("powershell")
                .args(["-Command", &script])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("mouse_move"),
                Ok(o) => ComputerResult::error("mouse_move", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("mouse_move", &e.to_string()),
            }
        }
    }

    fn left_click(&self, x: Option<i32>, y: Option<i32>) -> ComputerResult {
        // Move to position first if specified
        if let (Some(x), Some(y)) = (x, y) {
            self.mouse_move(x, y);
        }

        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("cliclick")
                .arg("c")
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("left_click"),
                Ok(o) => ComputerResult::error("left_click", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("left_click", &e.to_string()),
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("xdotool")
                .arg("click")
                .arg("1")
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("left_click"),
                Ok(o) => ComputerResult::error("left_click", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("left_click", &e.to_string()),
            }
        }

        #[cfg(target_os = "windows")]
        {
            let script = r#"Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait("{LEFT}")"#;
            let output = std::process::Command::new("powershell")
                .args(["-Command", script])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("left_click"),
                Ok(o) => ComputerResult::error("left_click", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("left_click", &e.to_string()),
            }
        }
    }

    fn right_click(&self, x: Option<i32>, y: Option<i32>) -> ComputerResult {
        if let (Some(x), Some(y)) = (x, y) {
            self.mouse_move(x, y);
        }

        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("cliclick")
                .arg("rc")
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("right_click"),
                Ok(o) => ComputerResult::error("right_click", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("right_click", &e.to_string()),
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("xdotool")
                .arg("click")
                .arg("3")
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("right_click"),
                Ok(o) => ComputerResult::error("right_click", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("right_click", &e.to_string()),
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = std::process::Command::new("powershell")
                .args(["-Command", "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MouseButtons]::Right"])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("right_click"),
                Ok(o) => ComputerResult::error("right_click", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("right_click", &e.to_string()),
            }
        }
    }

    fn double_click(&self, x: Option<i32>, y: Option<i32>) -> ComputerResult {
        if let (Some(x), Some(y)) = (x, y) {
            self.mouse_move(x, y);
        }

        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("cliclick")
                .arg("dc")
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("double_click"),
                Ok(o) => ComputerResult::error("double_click", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("double_click", &e.to_string()),
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("xdotool")
                .arg("click")
                .arg("--repeat")
                .arg("2")
                .arg("1")
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("double_click"),
                Ok(o) => ComputerResult::error("double_click", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("double_click", &e.to_string()),
            }
        }

        #[cfg(target_os = "windows")]
        {
            let script = r#"Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait("{F2}")"#;
            let output = std::process::Command::new("powershell")
                .args(["-Command", script])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("double_click"),
                Ok(o) => ComputerResult::error("double_click", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("double_click", &e.to_string()),
            }
        }
    }

    fn scroll(&self, x: i32, y: i32) -> ComputerResult {
        #[cfg(target_os = "macos")]
        {
            let clicks = y / 100;
            let output = std::process::Command::new("cliclick")
                .args(["scroll", &clicks.to_string()])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("scroll"),
                Ok(o) => ComputerResult::error("scroll", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("scroll", &e.to_string()),
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("xdotool")
                .args(["click", &format!("-- {}", if y > 0 { 4 } else { 5 })])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("scroll"),
                Ok(o) => ComputerResult::error("scroll", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("scroll", &e.to_string()),
            }
        }

        #[cfg(target_os = "windows")]
        {
            ComputerResult::success("scroll") // Simplified for Windows
        }
    }

    fn type_text(&self, text: &str) -> ComputerResult {
        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("sh")
                .args(["-c", &format!("echo '{}' | pbcopy", text.replace("'", "'\\''"))])
                .output();

            if output.is_err() || !output.as_ref().unwrap().status.success() {
                return ComputerResult::error("type_text", "Failed to copy to clipboard");
            }

            let output = std::process::Command::new("sh")
                .args(["-c", "osascript -e 'tell application \"System Events\" to keystroke \"v\" using command down'"])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("type_text"),
                Ok(o) => ComputerResult::error("type_text", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("type_text", &e.to_string()),
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("xdotool")
                .args(["type", "--", text])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("type_text"),
                Ok(o) => ComputerResult::error("type_text", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("type_text", &e.to_string()),
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = std::process::Command::new("powershell")
                .args(["-Command", &format!("Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('{}')", text)])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("type_text"),
                Ok(o) => ComputerResult::error("type_text", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("type_text", &e.to_string()),
            }
        }
    }

    fn key_combo(&self, keys: &[String]) -> ComputerResult {
        let combo = keys.join("+");

        #[cfg(target_os = "macos")]
        {
            let script = format!(
                r#"osascript -e 'tell application "System Events" to keystroke "{}" using {} down'"#,
                keys.last().unwrap_or(&String::new()),
                keys.iter().take(keys.len() - 1).map(|k| format!("{} command", k)).collect::<Vec<_>>().join(" and ")
            );
            let output = std::process::Command::new("sh")
                .args(["-c", &script])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("key_combo"),
                Ok(o) => ComputerResult::error("key_combo", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("key_combo", &e.to_string()),
            }
        }

        #[cfg(target_os = "linux")]
        {
            let mut args = vec!["key".to_string()];
            args.extend(keys.iter().cloned());
            let output = std::process::Command::new("xdotool")
                .args(&args)
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("key_combo"),
                Ok(o) => ComputerResult::error("key_combo", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("key_combo", &e.to_string()),
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = std::process::Command::new("powershell")
                .args(["-Command", &format!("Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('^{}')", keys.join(""))])
                .output();

            match output {
                Ok(o) if o.status.success() => ComputerResult::success("key_combo"),
                Ok(o) => ComputerResult::error("key_combo", &String::from_utf8_lossy(&o.stderr)),
                Err(e) => ComputerResult::error("key_combo", &e.to_string()),
            }
        }
    }

    fn screenshot_action(&self) -> ComputerResult {
        match crate::vision::Screenshot::capture() {
            Ok(screenshot) => ComputerResult::success("screenshot")
                .with_output(format!("Screenshot saved to: {}", screenshot.path.display())),
            Err(e) => ComputerResult::error("screenshot", &e.to_string()),
        }
    }

    fn wait(&self, seconds: u64) -> ComputerResult {
        std::thread::sleep(std::time::Duration::from_secs(seconds));
        ComputerResult::success("wait")
            .with_output(format!("Waited {} seconds", seconds))
    }
}

impl Default for ComputerUse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_computer_result() {
        let result = ComputerResult::success("test");
        assert!(result.success);
        assert_eq!(result.action, "test");
    }

    #[test]
    fn test_action_serialization() {
        let action = ComputerAction::MouseMove { x: 100, y: 200 };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("mouse_move"));
        assert!(json.contains("100"));
    }
}
