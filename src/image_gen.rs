//! Image Generation Tool
//!
//! Generate images using AI APIs (DALL-E, Stable Diffusion, etc.)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Image generation provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImageProvider {
    OpenAI,
    StabilityAI,
    Replicate,
    Local,
}

/// Image generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: u32,
    pub height: u32,
    pub steps: Option<u32>,
    pub guidance_scale: Option<f32>,
    pub seed: Option<u64>,
    pub model: Option<String>,
    pub provider: Option<String>,
}

impl Default for ImageRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            negative_prompt: None,
            width: 1024,
            height: 1024,
            steps: Some(30),
            guidance_scale: Some(7.5),
            seed: None,
            model: None,
            provider: None,
        }
    }
}

/// Image generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResult {
    pub success: bool,
    pub image_path: Option<String>,
    pub image_url: Option<String>,
    pub base64: Option<String>,
    pub prompt: String,
    pub model: String,
    pub generation_time_ms: u64,
    pub seed: Option<u64>,
    pub error: Option<String>,
}

/// Image generator
#[allow(dead_code)]
pub struct ImageGenerator {
    api_key: Option<String>,
    base_url: Option<String>,
}

impl ImageGenerator {
    pub fn new(api_key: Option<String>, base_url: Option<String>) -> Self {
        Self { api_key, base_url }
    }

    /// Generate an image
    pub async fn generate(&self, request: &ImageRequest) -> Result<ImageResult> {
        let start = std::time::Instant::now();

        // Determine provider
        let provider = request.provider.as_deref().unwrap_or("openai");

        let mut result = match provider {
            "openai" | "dalle" => self.generate_dalle(request).await,
            "stability" | "stabilityai" => self.generate_stability(request).await,
            "replicate" => self.generate_replicate(request).await,
            "local" => self.generate_local(request).await,
            _ => self.generate_dalle(request).await,
        }?;

        result.generation_time_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// Generate using DALL-E (OpenAI)
    async fn generate_dalle(&self, request: &ImageRequest) -> Result<ImageResult> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("OpenAI API key required for DALL-E"))?;

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.openai.com/v1/images/generations")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "prompt": request.prompt,
                "n": 1,
                "size": format!("{}x{}", request.width, request.height),
                "model": request.model.as_deref().unwrap_or("dall-e-3"),
                "response_format": "b64_json"
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Ok(ImageResult {
                success: false,
                image_path: None,
                image_url: None,
                base64: None,
                prompt: request.prompt.clone(),
                model: "dall-e-3".to_string(),
                generation_time_ms: 0,
                seed: request.seed,
                error: Some(error),
            });
        }

        let data: serde_json::Value = response.json().await?;

        // Safely access the first element of the data array
        let data_array = data["data"].as_array();
        let first_item = data_array.and_then(|arr| arr.first());

        let b64_data = first_item.and_then(|item| item["b64_json"].as_str());
        let image_url = first_item.and_then(|item| item["url"].as_str()).map(String::from);

        Ok(ImageResult {
            success: true,
            image_path: None,
            image_url,
            base64: b64_data.map(String::from),
            prompt: request.prompt.clone(),
            model: "dall-e-3".to_string(),
            generation_time_ms: 0,
            seed: request.seed,
            error: None,
        })
    }

    /// Generate using Stability AI
    async fn generate_stability(&self, _request: &ImageRequest) -> Result<ImageResult> {
        // Stub - would implement Stability AI API
        Ok(ImageResult {
            success: true,
            image_path: None,
            image_url: None,
            base64: None,
            prompt: _request.prompt.clone(),
            model: "stable-diffusion-xl".to_string(),
            generation_time_ms: 0,
            seed: _request.seed,
            error: None,
        })
    }

    /// Generate using Replicate
    async fn generate_replicate(&self, _request: &ImageRequest) -> Result<ImageResult> {
        // Stub - would implement Replicate API
        Ok(ImageResult {
            success: true,
            image_path: None,
            image_url: None,
            base64: None,
            prompt: _request.prompt.clone(),
            model: "stability-ai/sdxl".to_string(),
            generation_time_ms: 0,
            seed: _request.seed,
            error: None,
        })
    }

    /// Generate using local model (e.g., Stable Diffusion)
    async fn generate_local(&self, _request: &ImageRequest) -> Result<ImageResult> {
        // Stub - would use local inference
        Ok(ImageResult {
            success: false,
            image_path: None,
            image_url: None,
            base64: None,
            prompt: _request.prompt.clone(),
            model: "local".to_string(),
            generation_time_ms: 0,
            seed: _request.seed,
            error: Some("Local model not configured".to_string()),
        })
    }

    /// Save base64 image to file
    pub fn save_image(&self, base64_data: &str, output_path: &PathBuf) -> Result<()> {
        use base64::Engine;
        let data = base64::engine::general_purpose::STANDARD.decode(base64_data)?;
        std::fs::write(output_path, data)?;
        Ok(())
    }
}

/// Format image result as markdown
pub fn format_image_result(result: &ImageResult) -> String {
    let mut md = String::new();

    md.push_str("## Image Generation Result\n\n");
    md.push_str(&format!("- **Status:** {}\n", if result.success { "Success" } else { "Failed" }));
    md.push_str(&format!("- **Model:** {}\n", result.model));
    md.push_str(&format!("- **Prompt:** {}\n", result.prompt));
    md.push_str(&format!("- **Generation Time:** {}ms\n", result.generation_time_ms));

    if let Some(seed) = result.seed {
        md.push_str(&format!("- **Seed:** {}\n", seed));
    }

    if let Some(url) = &result.image_url {
        md.push_str(&format!("\n**Image URL:** {}\n", url));
    }

    if let Some(path) = &result.image_path {
        md.push_str(&format!("\n**Saved to:** {}\n", path));
    }

    if let Some(err) = &result.error {
        md.push_str(&format!("\n**Error:** {}\n", err));
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_request_default() {
        let req = ImageRequest::default();
        assert_eq!(req.width, 1024);
        assert_eq!(req.height, 1024);
    }
}
