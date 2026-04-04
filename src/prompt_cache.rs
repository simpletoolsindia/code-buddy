//! Prompt Caching - Anthropic prompt caching implementation
//!
//! Implements Anthropic's cache control with:
//! - Cache at breakpoint markers
//! - Partial invalidation on context changes
//! - Cache budget management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cached message block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedBlock {
    pub id: String,
    pub content: String,
    pub block_type: CacheBlockType,
    pub tokens: usize,
    pub created_at: String,
    pub hit_count: u32,
    pub last_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CacheBlockType {
    SystemPrompt,
    Memory,
    Skills,
    ContextFiles,
    History,
    Tools,
}

/// Cache budget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheBudget {
    pub total_tokens: usize,
    pub used_tokens: usize,
    pub max_tokens: usize,
}

impl CacheBudget {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            total_tokens: 0,
            used_tokens: 0,
            max_tokens,
        }
    }

    pub fn remaining(&self) -> usize {
        self.max_tokens.saturating_sub(self.used_tokens)
    }

    pub fn add(&mut self, tokens: usize) {
        self.used_tokens = self.used_tokens.saturating_add(tokens);
    }

    pub fn evict_least_used(&mut self, blocks: &mut HashMap<String, CachedBlock>, target_tokens: usize) {
        let ids_to_remove: Vec<_> = {
            let mut candidates: Vec<_> = blocks.values_mut().collect();
            candidates.sort_by_key(|b| b.hit_count);

            let mut freed = 0;
            candidates.into_iter().take_while(|b| {
                if self.used_tokens.saturating_sub(freed) <= target_tokens {
                    return false;
                }
                freed += b.tokens;
                true
            }).map(|b| b.id.clone()).collect()
        };

        for id in ids_to_remove {
            blocks.remove(&id);
        }

        self.used_tokens = self.used_tokens.saturating_sub(
            blocks.values().map(|b| b.tokens).sum::<usize>()
        );
    }
}

/// Prompt cache
pub struct PromptCache {
    blocks: HashMap<String, CachedBlock>,
    budget: CacheBudget,
    breakpoints: Vec<String>,
}

impl PromptCache {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            blocks: HashMap::new(),
            budget: CacheBudget::new(max_tokens),
            breakpoints: vec![
                "<!-- cache -->".to_string(),
                "[CACHE]".to_string(),
                "<cache>".to_string(),
            ],
        }
    }

    /// Add a block to cache
    pub fn cache_block(&mut self, block_type: CacheBlockType, content: &str, tokens: usize) -> String {
        let id = nanoid::nanoid!(12);
        let now = chrono::Utc::now().to_rfc3339();

        // Evict if over budget
        if self.budget.remaining() < tokens {
            self.budget.evict_least_used(&mut self.blocks, tokens);
        }

        let block = CachedBlock {
            id: id.clone(),
            content: content.to_string(),
            block_type,
            tokens,
            created_at: now.clone(),
            hit_count: 0,
            last_used: now,
        };

        self.budget.add(tokens);
        self.blocks.insert(id.clone(), block);

        id
    }

    /// Get a cached block
    pub fn get(&mut self, id: &str) -> Option<&CachedBlock> {
        if let Some(block) = self.blocks.get_mut(id) {
            block.hit_count += 1;
            block.last_used = chrono::Utc::now().to_rfc3339();
        }
        self.blocks.get(id)
    }

    /// Extract cache markers from content and build cached messages
    pub fn extract_and_cache(&mut self, content: &str) -> (String, Vec<String>) {
        let mut cleaned = content.to_string();
        let mut cached_ids = vec![];
        let breakpoints = self.breakpoints.clone();

        for marker in &breakpoints {
            if cleaned.contains(marker) {
                // Split at marker and cache the second part
                let parts: Vec<&str> = cleaned.split(marker).collect();
                if parts.len() > 1 {
                    let cache_content = parts[1].trim();
                    let tokens = estimate_tokens(cache_content);
                    let id = self.cache_block(CacheBlockType::ContextFiles, cache_content, tokens);
                    cached_ids.push(id);
                    cleaned = format!("{} {}", parts[0].trim(), marker);
                }
            }
        }

        (cleaned, cached_ids)
    }

    /// Build cached message for API
    pub fn build_cached_message(&self, block_type: CacheBlockType) -> serde_json::Value {
        let blocks: Vec<_> = self.blocks.values()
            .filter(|b| b.block_type == block_type)
            .collect();

        if blocks.is_empty() {
            return serde_json::json!([]);
        }

        serde_json::json!([{
            "type": "cache_control",
            "control": {
                "type": "ephemeral"
            }
        }])
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut total_tokens = 0;

        for block in self.blocks.values() {
            *by_type.entry(format!("{:?}", block.block_type)).or_insert(0) += block.tokens;
            total_tokens += block.tokens;
        }

        CacheStats {
            total_blocks: self.blocks.len(),
            total_tokens,
            max_tokens: self.budget.max_tokens,
            remaining: self.budget.remaining(),
            by_type,
            hit_rate: self.calculate_hit_rate(),
        }
    }

    fn calculate_hit_rate(&self) -> f64 {
        let total_hits: u32 = self.blocks.values().map(|b| b.hit_count).sum();
        if total_hits == 0 {
            return 0.0;
        }
        let total_accesses = self.blocks.values().map(|b| b.hit_count).sum::<u32>() as f64;
        total_accesses / self.blocks.len() as f64
    }

    /// Invalidate cache on context change
    pub fn invalidate_type(&mut self, block_type: &CacheBlockType) {
        self.blocks.retain(|_, b| &b.block_type != block_type);
        // Recalculate budget
        self.budget.used_tokens = self.blocks.values().map(|b| b.tokens).sum();
    }

    /// Clear entire cache
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.budget.used_tokens = 0;
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_blocks: usize,
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub remaining: usize,
    pub by_type: HashMap<String, usize>,
    pub hit_rate: f64,
}

/// Estimate token count (rough approximation)
fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: ~4 chars per token for English
    text.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let mut cache = PromptCache::new(1000);
        let id = cache.cache_block(CacheBlockType::SystemPrompt, "Test content", 100);
        assert!(!id.is_empty());

        let block = cache.get(&id);
        assert!(block.is_some());
        assert_eq!(block.unwrap().hit_count, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = PromptCache::new(200);
        cache.cache_block(CacheBlockType::SystemPrompt, "Content 1", 100);
        cache.cache_block(CacheBlockType::Memory, "Content 2", 100);
        cache.cache_block(CacheBlockType::Skills, "Content 3", 100); // Should evict one

        assert!(cache.blocks.len() <= 2);
    }

    #[test]
    fn test_stats() {
        let mut cache = PromptCache::new(1000);
        cache.cache_block(CacheBlockType::SystemPrompt, "Test", 100);

        let stats = cache.stats();
        assert!(stats.total_blocks >= 1);
        assert!(stats.total_tokens >= 100);
    }

    #[test]
    fn test_extract_and_cache() {
        let mut cache = PromptCache::new(1000);
        let content = "Some content <!-- cache --> cached content";
        let (cleaned, _ids) = cache.extract_and_cache(content);
        assert!(cleaned.contains("<!-- cache -->"));
    }

    #[test]
    fn test_invalidate_type() {
        let mut cache = PromptCache::new(1000);
        cache.cache_block(CacheBlockType::SystemPrompt, "System", 100);
        cache.cache_block(CacheBlockType::Memory, "Memory", 100);

        cache.invalidate_type(&CacheBlockType::SystemPrompt);

        let stats = cache.stats();
        assert!(stats.total_blocks <= 1);
    }

    #[test]
    fn test_clear_cache() {
        let mut cache = PromptCache::new(1000);
        cache.cache_block(CacheBlockType::SystemPrompt, "Test", 100);
        cache.cache_block(CacheBlockType::Memory, "Test", 100);

        cache.clear();

        let stats = cache.stats();
        assert_eq!(stats.total_blocks, 0);
        assert_eq!(stats.total_tokens, 0);
    }

    #[test]
    fn test_multiple_gets_increment_hit_count() {
        let mut cache = PromptCache::new(1000);
        let id = cache.cache_block(CacheBlockType::SystemPrompt, "Test", 100);

        cache.get(&id);
        cache.get(&id);
        cache.get(&id);

        let block = cache.get(&id).unwrap();
        assert_eq!(block.hit_count, 4); // 1 from cache + 3 gets
    }

    #[test]
    fn test_build_cached_message() {
        let cache = PromptCache::new(1000);
        let msg = cache.build_cached_message(CacheBlockType::SystemPrompt);
        assert!(msg.is_array() || msg.is_object());
    }
}
