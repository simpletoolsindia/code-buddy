//! Vision Support
//!
//! Provides screenshot and image analysis capabilities when the LLM supports vision.
//! - Screenshot capture
//! - Image analysis
//! - Screen reading for computer use

use anyhow::Result;
use std::path::PathBuf;

/// Supported vision models
pub const VISION_MODELS: &[&str] = &[
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    "claude-haiku-4-5",
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4-vision",
];

/// Check if the current model supports vision
pub fn model_supports_vision(model: Option<&str>) -> bool {
    match model {
        Some(m) => VISION_MODELS.iter().any(|v| m.contains(v)),
        None => true, // Assume support if no specific model
    }
}

/// Screenshot capture result
#[derive(Debug, Clone)]
pub struct Screenshot {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub size_bytes: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Screenshot {
    /// Capture a screenshot using platform-specific tools
    pub fn capture() -> Result<Self> {
        let timestamp = chrono::Utc::now();
        let filename = format!("screenshot_{}.png", timestamp.format("%Y%m%d_%H%M%S"));

        let output_path = std::env::temp_dir().join(&filename);
        let output_path_str = output_path.to_string_lossy().into_owned();

        #[cfg(target_os = "macos")]
        {
            // Use screencapture on macOS
            let output = std::process::Command::new("screencapture")
                .args(["-x", &output_path_str])
                .output()?;

            if !output.status.success() {
                anyhow::bail!("screencapture failed");
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Try gnome-screenshot, scrot, or grim
            let result = std::process::Command::new("gnome-screenshot")
                .args(["-f", &output_path_str])
                .output();

            if result.is_err() || !result.as_ref().unwrap().status.success() {
                // Try scrot
                let result = std::process::Command::new("scrot")
                    .arg(&output_path_str)
                    .output();

                if result.is_err() || !result.as_ref().unwrap().status.success() {
                    // Try grim (for Wayland)
                    let result = std::process::Command::new("grim")
                        .arg(&output_path_str)
                        .output()?;

                    if !result.status.success() {
                        anyhow::bail!("No screenshot tool available (tried: gnome-screenshot, scrot, grim)");
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Use PowerShell on Windows
            let script = format!(
                r#"Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Screen]::PrimaryScreen | ForEach-Object {{ [System.Drawing.Bitmap].new($_.Bounds.Width, $_.Bounds.Height) | ForEach-Object {{ $g = [System.Drawing.Graphics]::FromImage($_); $g.CopyFromScreen($_.Bounds.Location, [System.Drawing.Point]::Empty, $_.Size); $_.Save("{}", [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose() }} }}"#,
                output_path_str.replace("\\", "\\\\")
            );

            let output = std::process::Command::new("powershell")
                .args(["-Command", &script])
                .output()?;

            if !output.status.success() {
                anyhow::bail!("Failed to capture screenshot on Windows");
            }
        }

        let metadata = std::fs::metadata(&output_path)?;
        let size_bytes = metadata.len() as usize;

        // Get image dimensions (placeholder - would need image crate for actual parsing)
        Ok(Screenshot {
            path: output_path,
            width: 0, // Would need image crate
            height: 0,
            size_bytes,
            timestamp,
        })
    }

    /// Convert to base64 for API submission
    pub fn to_base64(&self) -> Result<String> {
        use std::io::Read;
        let mut file = std::fs::File::open(&self.path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(base64_encode(&buffer))
    }

    /// Get as data URL
    pub fn to_data_url(&self) -> Result<String> {
        let b64 = self.to_base64()?;
        Ok(format!("data:image/png;base64,{}", b64))
    }
}

/// Simple base64 encoder
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(CHARS[b0 >> 2] as char);
        result.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Image info for API
#[derive(Debug, Clone)]
pub struct VisionImage {
    pub source: VisionSource,
    pub media_type: String,
}

#[derive(Debug, Clone)]
pub enum VisionSource {
    /// Screenshot capture
    Screenshot,
    /// File path
    Path(PathBuf),
    /// Base64 encoded
    Base64(String),
    /// URL
    Url(String),
}

impl VisionImage {
    /// Create from screenshot
    pub fn from_screenshot() -> Result<Self> {
        let screenshot = Screenshot::capture()?;
        Ok(Self {
            source: VisionSource::Screenshot,
            media_type: "image/png".to_string(),
        })
    }

    /// Create from file path
    pub fn from_path(path: &PathBuf) -> Result<Self> {
        Ok(Self {
            source: VisionSource::Path(path.clone()),
            media_type: mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string(),
        })
    }

    /// Create from URL
    pub fn from_url(url: &str) -> Self {
        Self {
            source: VisionSource::Url(url.to_string()),
            media_type: "image/jpeg".to_string(), // Assume JPEG unless specified
        }
    }
}

/// Vision request for API
pub struct VisionRequest {
    pub prompt: String,
    pub images: Vec<VisionImage>,
}

impl VisionRequest {
    pub fn new(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            images: Vec::new(),
        }
    }

    pub fn with_screenshot(mut self) -> Result<Self> {
        self.images.push(VisionImage::from_screenshot()?);
        Ok(self)
    }

    pub fn with_image(mut self, path: PathBuf) -> Result<Self> {
        self.images.push(VisionImage::from_path(&path)?);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_models() {
        assert!(model_supports_vision(Some("claude-opus-4-6")));
        assert!(model_supports_vision(Some("claude-sonnet-4-6")));
        assert!(model_supports_vision(Some("gpt-4o")));
        assert!(!model_supports_vision(Some("gpt-3.5-turbo")));
    }

    #[test]
    fn test_base64_encode() {
        let encoded = base64_encode(b"Hello");
        assert_eq!(encoded, "SGVsbG8=");
    }
}
