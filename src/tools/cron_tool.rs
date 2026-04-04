//! Cron Tool - Schedule recurring tasks, reminders, and delayed actions
//!
//! Schedule format examples:
//!   - "30m"     → every 30 minutes
//!   - "2h"      → every 2 hours
//!   - "1d"      → every 1 day
//!   - "0 9 * * *" → standard cron expression (every day at 9am)
//!   - "+1h"     → one-shot, run 1 hour from now

use anyhow::Result;
use crate::cron;
use serde::{Deserialize, Serialize};

use super::Tool;

/// Cron tool for managing scheduled jobs
pub struct CronTool;

impl CronTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CronTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for CronTool {
    fn name(&self) -> &str {
        "Cron"
    }

    fn description(&self) -> &str {
        "Manage scheduled and recurring tasks. \
Create scheduled jobs with intervals (30m, 2h, 1d), cron expressions (0 9 * * *), \
or delays (+1h). List, pause, resume, and delete scheduled jobs. \
Use for reminders, periodic health checks, report generation, and deferred tasks. \
Args: <action> [args...]
  list                    - List all scheduled jobs
  create <schedule> <prompt> - Create a new scheduled job
  delete <job_id>         - Delete a scheduled job
  pause <job_id>          - Pause a scheduled job
  resume <job_id>          - Resume a paused job
  trigger <job_id>         - Trigger a job to run immediately
Example: Cron('create 30m Check disk space')
Example: Cron('create 0 9 * * * Morning health check')
Example: Cron('list')"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Cron tool usage:\n\
  list                    - List all scheduled jobs\n\
  create <schedule> <prompt> - Create a new scheduled job\n\
  delete <job_id>         - Delete a scheduled job\n\
  pause <job_id>          - Pause a scheduled job\n\
  resume <job_id>         - Resume a paused job\n\
  trigger <job_id>        - Trigger a job immediately\n\
Schedules: 30m, 2h, 1d, +1h, 0 9 * * * (cron)".to_string());
        }

        let action = &args[0].to_lowercase();
        match action.as_str() {
            "list" => {
                let result = cron::list_jobs()?;
                let output = serde_json::to_string_pretty(&serde_json::json!({
                    "success": result.success,
                    "count": result.count,
                    "jobs": result.jobs,
                    "message": result.message,
                }))?;
                Ok(output)
            }
            "create" => {
                if args.len() < 3 {
                    return Ok("Usage: Cron('create', '<schedule>', '<prompt>')\n\
Schedules: 30m, 2h, 1d, +1h, 0 9 * * *".to_string());
                }
                let schedule_str = &args[1];
                let prompt = &args[2..].join(" ");
                let result = cron::create_job(
                    prompt,
                    schedule_str,
                    None,
                    None,
                    vec![],
                    Some("local"),
                    None,
                    None,
                    None,
                )?;
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": result.success,
                    "job_id": result.job_id,
                    "name": result.name,
                    "schedule": result.schedule,
                    "next_run_at": result.next_run_at,
                    "message": result.message,
                    "error": result.error,
                }))?)
            }
            "delete" => {
                if args.len() < 2 {
                    return Ok("Usage: Cron('delete', '<job_id>')".to_string());
                }
                let job_id = &args[1];
                let result = cron::remove_job(job_id)?;
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": result.success,
                    "job_id": job_id,
                    "message": result.message,
                    "error": result.error,
                }))?)
            }
            "pause" => {
                if args.len() < 2 {
                    return Ok("Usage: Cron('pause', '<job_id>')".to_string());
                }
                let job_id = &args[1];
                let result = cron::pause_job(job_id, None)?;
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": result.success,
                    "job_id": job_id,
                    "message": result.message,
                    "error": result.error,
                }))?)
            }
            "resume" => {
                if args.len() < 2 {
                    return Ok("Usage: Cron('resume', '<job_id>')".to_string());
                }
                let job_id = &args[1];
                let result = cron::resume_job(job_id)?;
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": result.success,
                    "job_id": job_id,
                    "message": result.message,
                    "error": result.error,
                }))?)
            }
            "trigger" => {
                if args.len() < 2 {
                    return Ok("Usage: Cron('trigger', '<job_id>')".to_string());
                }
                let job_id = &args[1];
                let result = cron::trigger_job(job_id)?;
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": result.success,
                    "job_id": job_id,
                    "message": result.message,
                    "error": result.error,
                }))?)
            }
            _ => {
                Ok(format!("Unknown action: {}\n\
Actions: list, create, delete, pause, resume, trigger", action))
            }
        }
    }
}
