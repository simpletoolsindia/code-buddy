//! Plan Command - Structured planning with steps
//!
//! Provides structured planning output with steps, risks, and dependencies.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Plan step status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

/// Plan step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: usize,
    pub description: String,
    pub status: StepStatus,
    pub notes: Option<String>,
    pub depends_on: Vec<usize>,
}

impl PlanStep {
    pub fn new(id: usize, description: &str) -> Self {
        Self {
            id,
            description: description.to_string(),
            status: StepStatus::Pending,
            notes: None,
            depends_on: Vec::new(),
        }
    }

    pub fn with_notes(mut self, notes: &str) -> Self {
        self.notes = Some(notes.to_string());
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<usize>) -> Self {
        self.depends_on = deps;
        self
    }
}

/// Plan risk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRisk {
    pub description: String,
    pub severity: RiskSeverity,
    pub mitigation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskSeverity {
    pub fn icon(&self) -> &str {
        match self {
            RiskSeverity::Low => "🟢",
            RiskSeverity::Medium => "🟡",
            RiskSeverity::High => "🟠",
            RiskSeverity::Critical => "🔴",
        }
    }
}

/// Plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub title: String,
    pub description: Option<String>,
    pub steps: Vec<PlanStep>,
    pub risks: Vec<PlanRisk>,
    pub prerequisites: Vec<String>,
    pub estimated_complexity: Option<String>,
}

impl Plan {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            description: None,
            steps: Vec::new(),
            risks: Vec::new(),
            prerequisites: Vec::new(),
            estimated_complexity: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn add_step(&mut self, description: &str) -> &PlanStep {
        let id = self.steps.len() + 1;
        self.steps.push(PlanStep::new(id, description));
        self.steps.last().unwrap()
    }

    pub fn add_risk(&mut self, risk: PlanRisk) {
        self.risks.push(risk);
    }

    pub fn add_prerequisite(&mut self, prereq: &str) {
        self.prerequisites.push(prereq.to_string());
    }

    pub fn completed_steps(&self) -> usize {
        self.steps.iter().filter(|s| s.status == StepStatus::Completed).count()
    }

    pub fn progress_percent(&self) -> f64 {
        if self.steps.is_empty() {
            return 100.0;
        }
        (self.completed_steps() as f64 / self.steps.len() as f64) * 100.0
    }
}

/// Format plan as markdown
pub fn format_plan(plan: &Plan) -> String {
    let mut md = format!("# {}\n\n", plan.title);

    if let Some(ref desc) = plan.description {
        md.push_str(&format!("{}\n\n", desc));
    }

    if !plan.prerequisites.is_empty() {
        md.push_str("## Prerequisites\n\n");
        for prereq in &plan.prerequisites {
            md.push_str(&format!("- [ ] {}\n", prereq));
        }
        md.push('\n');
    }

    if !plan.steps.is_empty() {
        md.push_str("## Steps\n\n");
        for step in &plan.steps {
            let checkbox = match step.status {
                StepStatus::Pending => "[ ]",
                StepStatus::InProgress => "[~]",
                StepStatus::Completed => "[x]",
                StepStatus::Skipped => "[-]",
            };
            let indent = if step.depends_on.is_empty() {
                String::new()
            } else {
                "  ".to_string()
            };
            md.push_str(&format!(
                "{}- {} **{}**",
                indent,
                checkbox,
                step.description
            ));
            if !step.depends_on.is_empty() {
                md.push_str(&format!(" (depends on #{})", step.depends_on.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", #")));
            }
            if let Some(ref notes) = step.notes {
                md.push_str(&format!("\n  {}", notes));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    if !plan.risks.is_empty() {
        md.push_str("## Risks\n\n");
        for risk in &plan.risks {
            md.push_str(&format!(
                "- {} **{}**: {}",
                risk.severity.icon(),
                format!("{:?}", risk.severity),
                risk.description
            ));
            if let Some(ref mitigation) = risk.mitigation {
                md.push_str(&format!("\n  - Mitigation: {}", mitigation));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    // Progress summary
    md.push_str(&format!(
        "---\n**Progress:** {}% ({}/{} steps)\n",
        plan.progress_percent() as usize,
        plan.completed_steps(),
        plan.steps.len()
    ));

    md
}

/// Generate a plan from a task description
pub fn generate_plan(task: &str) -> Plan {
    let mut plan = Plan::new(task);

    // Extract key phrases to understand the task
    let task_lower = task.to_lowercase();

    if task_lower.contains("implement") || task_lower.contains("create") || task_lower.contains("add") {
        plan.add_step("Understand requirements and specifications");
        plan.add_step("Design the solution");
        plan.add_step("Write the implementation");
        plan.add_step("Add tests");
        plan.add_step("Update documentation");
        plan.add_prerequisite("Development environment set up");
    } else if task_lower.contains("fix") || task_lower.contains("bug") || task_lower.contains("repair") {
        plan.add_step("Reproduce the issue");
        plan.add_step("Identify the root cause");
        plan.add_step("Implement the fix");
        plan.add_step("Verify the fix works");
        plan.add_step("Check for regressions");
    } else if task_lower.contains("refactor") || task_lower.contains("improve") || task_lower.contains("optimize") {
        plan.add_step("Identify code to refactor");
        plan.add_step("Plan refactoring approach");
        plan.add_step("Make incremental changes");
        plan.add_step("Run tests to verify behavior");
        plan.add_prerequisite("Good test coverage");
    } else {
        // Default plan
        plan.add_step("Analyze the task");
        plan.add_step("Plan the approach");
        plan.add_step("Execute the plan");
        plan.add_step("Review and verify results");
    }

    plan
}

/// Run plan command
pub fn run(args: &[String]) -> anyhow::Result<String> {
    if args.is_empty() {
        return Ok(r#"# Plan Command

Create implementation plans for tasks.

## Usage

```
plan <task description>     Create a plan for the task
plan --interactive          Create plan interactively
plan --simple <task>       Create a simple plan
```

## Examples

```
plan "implement user authentication"
plan "fix the login bug"
plan "refactor the database layer"
```

## Features

- Automatic task type detection
- Risk assessment
- Dependency tracking
- Progress estimation
"#.to_string());
    }

    let task = args.join(" ");
    let plan = generate_plan(&task);
    Ok(format_plan(&plan))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_creation() {
        let mut plan = Plan::new("Test Plan");
        plan.add_step("Step 1");
        plan.add_step("Step 2");
        assert_eq!(plan.steps.len(), 2);
    }

    #[test]
    fn test_plan_progress() {
        let mut plan = Plan::new("Test");
        plan.add_step("Step 1");
        plan.add_step("Step 2");
        assert_eq!(plan.progress_percent(), 0.0);
    }

    #[test]
    fn test_generate_plan_for_fix() {
        let plan = generate_plan("Fix the login bug");
        assert!(plan.title.contains("login bug"));
        assert!(!plan.steps.is_empty());
    }

    #[test]
    fn test_plan_format() {
        let mut plan = Plan::new("Test Plan");
        plan.add_step("Do something");
        plan.add_risk(PlanRisk {
            description: "Risk 1".to_string(),
            severity: RiskSeverity::Medium,
            mitigation: Some("Mitigation 1".to_string()),
        });
        let md = format_plan(&plan);
        assert!(md.contains("# Test Plan"));
        assert!(md.contains("## Steps"));
        assert!(md.contains("## Risks"));
    }
}
