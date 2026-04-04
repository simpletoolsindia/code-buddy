//! Image Tool - Generate images using AI
//!
//! Supports OpenAI DALL-E, Stability AI, and local models.
//! Use for architecture diagrams, UI concepts, banners, illustrations.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use super::Tool;

/// Image generation tool
pub struct ImageTool;

impl ImageTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ImageTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ImageTool {
    fn name(&self) -> &str {
        "ImageGenerate"
    }

    fn description(&self) -> &str {
        "Generate images using AI. \
Supports OpenAI DALL-E (default), Stability AI, Replicate. \
Use for architecture diagrams, UI mockups, banners, illustrations, concept art. \
Args: <prompt> [--provider <name>] [--width <px>] [--height <px>] [--steps <n>]
Example: ImageGenerate('A futuristic city at sunset')
Example: ImageGenerate('Clean architecture diagram with boxes and arrows', '--width 1024 --height 512')
Example: ImageGenerate('Dark mode dashboard UI', '--provider openai --width 1920 --height 1080')"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("ImageGenerate tool usage:\n\
  ImageGenerate(<prompt>, [--provider openai|stable|replicate>], [--width <px>], [--height <px>])\n\
  Default provider: openai (DALL-E)\n\
  Default size: 1024x1024\n\
  Returns: image path/URL or base64 data\n\
  Note: Requires API key configured".to_string());
        }

        let prompt = args.first().map(|s| s.as_str()).unwrap_or("");

        // Parse optional flags
        let mut provider = "openai".to_string();
        let mut width: u32 = 1024;
        let mut height: u32 = 1024;

        for i in 1..args.len() {
            match args[i].as_str() {
                "--provider" if i + 1 < args.len() => { provider = args[i + 1].clone(); }
                "--width" if i + 1 < args.len() => { width = args[i + 1].parse().unwrap_or(1024); }
                "--height" if i + 1 < args.len() => { height = args[i + 1].parse().unwrap_or(1024); }
                _ => {}
            }
        }

        let request = code_buddy::image_gen::ImageRequest {
            prompt: prompt.to_string(),
            width,
            height,
            provider: Some(provider.clone()),
            ..Default::default()
        };

        let generator = code_buddy::image_gen::ImageGenerator::new(None, None);

        // Run async via block_on
        let rt = tokio::runtime::Runtime::new()?;
        let result = rt.block_on(generator.generate(&request));

        match result {
            Ok(img_result) => {
                let output = serde_json::to_string_pretty(&serde_json::json!({
                    "success": img_result.success,
                    "prompt": img_result.prompt,
                    "image_url": img_result.image_url,
                    "image_path": img_result.image_path,
                    "base64_preview": img_result.base64.as_ref().map(|b| format!("{}...", &b[..b.len().min(100)])),
                    "model": img_result.model,
                    "generation_time_ms": img_result.generation_time_ms,
                    "seed": img_result.seed,
                    "error": img_result.error,
                }))?;
                Ok(output)
            }
            Err(e) => {
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                    "hint": "Set OPENAI_API_KEY or configure image generation provider"
                }))?)
            }
        }
    }
}
