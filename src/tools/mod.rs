//! Tool implementations

pub mod bash;
pub mod executor;
pub mod file;
pub mod grep;
pub mod glob;
pub mod web;

#[cfg(test)]
mod tests;

use anyhow::Result;

/// Tool trait for implementing Claude Code tools
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &[String]) -> Result<String>;
}

/// Async tool trait
pub trait AsyncTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &[String]) -> impl std::future::Future<Output = Result<String>> + Send;
}
