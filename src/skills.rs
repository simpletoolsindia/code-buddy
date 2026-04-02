//! Skills System
//!
//! Provides extensible skill system for Code Buddy.
//! Skills are modular capabilities that can be invoked via slash commands.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    pub instructions: String,
    pub tools: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillCategory {
    #[serde(rename = "workflow")]
    Workflow,
    #[serde(rename = "code")]
    Code,
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "test")]
    Test,
    #[serde(rename = "security")]
    Security,
    #[serde(rename = "devops")]
    DevOps,
    #[serde(rename = "database")]
    Database,
    #[serde(rename = "documentation")]
    Documentation,
    #[serde(rename = "custom")]
    Custom,
}

impl SkillCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillCategory::Workflow => "workflow",
            SkillCategory::Code => "code",
            SkillCategory::Debug => "debug",
            SkillCategory::Test => "test",
            SkillCategory::Security => "security",
            SkillCategory::DevOps => "devops",
            SkillCategory::Database => "database",
            SkillCategory::Documentation => "documentation",
            SkillCategory::Custom => "custom",
        }
    }
}

impl std::fmt::Display for SkillCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Skill {
    pub fn new(id: &str, name: &str, description: &str, category: SkillCategory) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            category,
            instructions: String::new(),
            tools: Vec::new(),
            enabled: true,
        }
    }

    pub fn with_instructions(mut self, instructions: &str) -> Self {
        self.instructions = instructions.to_string();
        self
    }

    pub fn with_tools(mut self, tools: Vec<&str>) -> Self {
        self.tools = tools.into_iter().map(String::from).collect();
        self
    }
}

/// Built-in skills
pub fn builtin_skills() -> Vec<Skill> {
    vec![
        // Code Quality Skills
        Skill::new("simplify", "Simplify", "Review code for quality, reuse, and efficiency", SkillCategory::Code)
            .with_instructions(r#"# /simplify - Code Quality Review

Review code for:
1. **Code Reuse** - Identify duplicated patterns that could be refactored
2. **Quality Issues** - Find potential bugs, anti-patterns, and code smells
3. **Efficiency** - Suggest performance improvements

## How to Use
/simplify             - Review all changed files
/simplify <file>     - Review specific file
/simplify <dir>      - Review directory"#)
            .with_tools(vec!["Read", "Grep", "Glob"]),

        Skill::new("review", "Review", "Full code review with comprehensive checklist", SkillCategory::Code)
            .with_instructions(r#"# /review - Full Code Review

## Review Checklist
- [ ] Correctness - Does the code work as intended?
- [ ] Security - Any security vulnerabilities?
- [ ] Performance - Any bottlenecks?
- [ ] Maintainability - Is the code easy to understand?
- [ ] Testing - Adequate test coverage?
- [ ] Documentation - Is the code documented?
- [ ] Error Handling - Are errors handled properly?
- [ ] Edge Cases - Are boundary conditions handled?

## Usage
/review changes     - Review git changes
/review all         - Full codebase review
/review <file>      - Review specific file"#)
            .with_tools(vec!["Read", "Bash", "Grep"]),

        // TDD Skill
        Skill::new("tdd", "TDD", "Test-driven development workflow", SkillCategory::Test)
            .with_instructions(r#"# /tdd - Test-Driven Development

Follow the TDD cycle:
1. **Red** - Write a failing test
2. **Green** - Write minimal code to pass
3. **Refactor** - Improve code while keeping tests passing

## Workflow
1. Write failing test: /tdd red <test description>
2. Make it pass: /tdd green
3. Refactor: /tdd refactor

## Example
```
/tdd red "should add two numbers"
# Write: fn test_add_two_numbers() { assert_eq!(add(2, 2), 4); }
/tdd green
# Write: fn add(a: i32, b: i32) -> i32 { a + b }
/tdd refactor
# Improve: Add documentation, handle edge cases
```"#)
            .with_tools(vec!["Read", "Write", "Bash"]),

        // Debug Skill
        Skill::new("debug", "Debug", "Systematic debugging assistance", SkillCategory::Debug)
            .with_instructions(r#"# /debug - Debug Assistant

## Debugging Process
1. **Understand** - What's the expected behavior?
2. **Reproduce** - Can you reproduce the bug consistently?
3. **Hypothesize** - What's causing the bug?
4. **Test** - Verify your hypothesis
5. **Fix** - Implement the fix
6. **Verify** - Ensure the bug is fixed

## Commands
/debug analyze <error>     - Analyze error message
/debug trace <function>    - Add tracing to function
/debug test <scenario>     - Write test for bug scenario"#)
            .with_tools(vec!["Read", "Bash", "Grep"]),

        // Batch Skill
        Skill::new("batch", "Batch", "Execute parallel tasks across codebase", SkillCategory::Workflow)
            .with_instructions(r#"# /batch - Batch Operations

Execute operations across multiple files or components.

## Usage
/batch refactor <pattern> <replacement>
  - Refactor all matching patterns

/batch test <pattern>
  - Run tests matching pattern

/batch update <files>
  - Update multiple files

## Example
/batch refactor "console.log" "logger.info"
  - Replace all console.log with logger.info"#)
            .with_tools(vec!["Read", "Write", "Edit", "Bash"]),

        // Security Skill
        Skill::new("security", "Security", "Security analysis and hardening", SkillCategory::Security)
            .with_instructions(r#"# /security - Security Analysis

## Security Checklist
- [ ] SQL Injection - Are queries parameterized?
- [ ] XSS - Is user input sanitized?
- [ ] Authentication - Is auth properly implemented?
- [ ] Authorization - Are permissions checked?
- [ ] Secrets - Are secrets in environment/config?
- [ ] Dependencies - Are there known vulnerabilities?

## Commands
/security scan        - Scan for common vulnerabilities
/security audit       - Full security audit
/security fix <type>  - Fix specific vulnerability type"#)
            .with_tools(vec!["Read", "Grep", "Bash"]),

        // DevOps Skill
        Skill::new("devops", "DevOps", "CI/CD and deployment assistance", SkillCategory::DevOps)
            .with_instructions(r#"# /devops - DevOps Helper

## Capabilities
- Docker configuration
- CI/CD pipeline setup
- Kubernetes manifests
- Monitoring and logging
- Deployment strategies

## Commands
/devops docker <action>  - Docker operations
/devops ci <action>      - CI/CD setup
/devops k8s <action>     - Kubernetes operations
/devops deploy <target>   - Deploy to target"#)
            .with_tools(vec!["Read", "Write", "Bash", "Glob"]),

        // Database Skill
        Skill::new("database", "Database", "Database design and query assistance", SkillCategory::Database)
            .with_instructions(r#"# /database - Database Helper

## Capabilities
- Schema design
- Query optimization
- Migration scripts
- ORM configuration

## Commands
/database design <spec>   - Design schema
/database migrate <old> <new>  - Generate migration
/database optimize <query> - Optimize query
/database seed <schema>   - Generate seed data"#)
            .with_tools(vec!["Read", "Write", "Bash"]),

        // Documentation Skill
        Skill::new("docs", "Docs", "Documentation generation and improvement", SkillCategory::Documentation)
            .with_instructions(r#"# /docs - Documentation Helper

## Capabilities
- API documentation
- README files
- Code comments
- Change logs

## Commands
/docs api <file>      - Generate API docs
/docs readme <project> - Generate README
/docs changelog       - Generate changelog
/docs comments        - Improve code comments"#)
            .with_tools(vec!["Read", "Write", "Glob"]),

        // Remember Skill
        Skill::new("remember", "Remember", "Save information to memory for later recall", SkillCategory::Workflow)
            .with_instructions(r#"# /remember - Persistent Memory

Save information to memory that persists across sessions.

## Usage
/remember <text>         - Save text to memory
/remember list            - List all memories
/remember search <query>  - Search memories
/remember delete <id>     - Delete a memory

## Examples
/remember API endpoint is https://api.example.com
/remember Project uses PostgreSQL 15
/remember list

## Notes
- Memories persist across sessions
- Search by keyword to find related information
- Use clear names for easy recall"#)
            .with_tools(vec!["Read", "Write"]),

        // Loop Skill
        Skill::new("loop", "Loop", "Run a command or task on a recurring interval", SkillCategory::Workflow)
            .with_instructions(r#"# /loop - Recurring Task Loop

Run a task or command on a recurring interval.

## Usage
/loop <interval> <command>  - Run command every interval
/loop status               - Show running loops
/loop stop <id>           - Stop a loop

## Intervals
- 5m, 10m, 30m - Minutes
- 1h, 2h, 6h   - Hours
- daily         - Once per day

## Examples
/loop 5m /status           - Check status every 5 minutes
/loop 1h /health-check     - Health check every hour
/loop daily /backup         - Daily backup

## Notes
- Loops run in the background
- Use /loop stop to cancel
- Maximum 5 concurrent loops"#)
            .with_tools(vec!["Bash", "Read"]),

        // Verify Skill
        Skill::new("verify", "Verify", "Verify code correctness and test coverage", SkillCategory::Test)
            .with_instructions(r#"# /verify - Code Verification

Verify code changes are correct and well-tested.

## Usage
/verify                   - Verify all changes
/verify <file>           - Verify specific file
/verify tests           - Run and verify tests
/verify coverage        - Check test coverage

## Checklist
- [ ] Code compiles without errors
- [ ] Tests pass
- [ ] No regression in existing functionality
- [ ] Code follows style guidelines
- [ ] Documentation updated if needed

## Examples
/verify src/main.rs
/verify tests
/verify coverage"#)
            .with_tools(vec!["Bash", "Read", "Glob"]),

        // Schedule Remote Agents Skill
        Skill::new("schedule", "Schedule", "Schedule agents to run at specific times", SkillCategory::Workflow)
            .with_instructions(r#"# /schedule - Schedule Remote Agents

Schedule agents to execute tasks at specific times.

## Usage
/schedule <time> <task>   - Schedule a task
/schedule list             - List scheduled tasks
/schedule cancel <id>      - Cancel scheduled task

## Examples
/schedule "tomorrow 9am" Run tests
/schedule "every monday" Generate report
/schedule "in 2 hours" Deploy staging

## Notes
- Times are in local timezone
- Tasks run even if you're not present
- Results saved to log file"#)
            .with_tools(vec!["Bash", "Read"]),
    ]
}

/// Skills manager
pub struct SkillsManager {
    skills: Vec<Skill>,
}

impl SkillsManager {
    pub fn new() -> Self {
        Self {
            skills: builtin_skills(),
        }
    }

    /// Load custom skills from directory
    pub fn load_from_dir(&mut self, dir: &PathBuf) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(skill) = self.parse_skill_from_md(&path, &content) {
                        self.skills.push(skill);
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse skill from markdown file
    fn parse_skill_from_md(&self, path: &PathBuf, content: &str) -> Option<Skill> {
        let name = path.file_stem()?.to_str()?;

        // Extract description from first paragraph
        let description = content
            .lines()
            .skip_while(|l| l.starts_with('#'))
            .skip_while(|l| l.trim().is_empty())
            .take_while(|l| !l.starts_with('#') && !l.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        Some(Skill::new(name, name, &description, SkillCategory::Custom)
            .with_instructions(content))
    }

    /// Get all skills
    pub fn get_all(&self) -> &[Skill] {
        &self.skills
    }

    /// Get enabled skills
    pub fn get_enabled(&self) -> Vec<&Skill> {
        self.skills.iter().filter(|s| s.enabled).collect()
    }

    /// Get skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.name == name || s.id == name)
    }

    /// Get skills by category
    pub fn get_by_category(&self, category: &SkillCategory) -> Vec<&Skill> {
        self.skills.iter().filter(|s| s.category == *category).collect()
    }

    /// Search skills
    pub fn search(&self, query: &str) -> Vec<&Skill> {
        let query_lower = query.to_lowercase();
        self.skills.iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.description.to_lowercase().contains(&query_lower)
                    || s.category.as_str().contains(&query_lower)
            })
            .collect()
    }

    /// Enable/disable skill
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(skill) = self.skills.iter_mut().find(|s| s.name == name || s.id == name) {
            skill.enabled = enabled;
            true
        } else {
            false
        }
    }
}

impl Default for SkillsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Format skills as markdown list
pub fn format_skills_list(skills: &[&Skill]) -> String {
    let mut md = String::from("# Available Skills\n\n");

    let mut categories: HashMap<&str, Vec<&Skill>> = HashMap::new();
    for skill in skills {
        let cat_name = match skill.category {
            SkillCategory::Workflow => "Workflow",
            SkillCategory::Code => "Code Quality",
            SkillCategory::Debug => "Debugging",
            SkillCategory::Test => "Testing",
            SkillCategory::Security => "Security",
            SkillCategory::DevOps => "DevOps",
            SkillCategory::Database => "Database",
            SkillCategory::Documentation => "Documentation",
            SkillCategory::Custom => "Custom",
        };
        categories.entry(cat_name).or_default().push(skill);
    }

    for (cat, skills) in categories {
        md.push_str(&format!("## {}\n\n", cat));
        for skill in skills {
            md.push_str(&format!("- `/{}` - {}\n", skill.name, skill.description));
        }
        md.push('\n');
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_skills() {
        let skills = builtin_skills();
        assert!(!skills.is_empty());
        assert!(skills.iter().any(|s| s.id == "simplify"));
        assert!(skills.iter().any(|s| s.id == "review"));
        assert!(skills.iter().any(|s| s.id == "tdd"));
    }

    #[test]
    fn test_skills_manager() {
        let manager = SkillsManager::new();
        assert!(manager.get("simplify").is_some());
        assert!(manager.search("debug").len() > 0);
        assert_eq!(manager.get_by_category(&SkillCategory::Code).len(), 2);
    }

    #[test]
    fn test_skill_toggle() {
        let mut manager = SkillsManager::new();
        assert!(manager.set_enabled("simplify", false));
        assert!(!manager.get("simplify").unwrap().enabled);
        assert!(manager.set_enabled("simplify", true));
    }
}
