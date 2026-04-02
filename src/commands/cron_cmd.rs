//! Cron Command - Built-in cron scheduler
//!
//! Provides scheduled task execution with natural language scheduling.

use crate::cron;
use crate::cron::{CronJobOutput, CronJobSummary};
use anyhow::Result;

/// Run cron command
pub fn run(args: &[String]) -> Result<String> {
    let mut args = args.to_vec();

    if args.is_empty() {
        return show_cron_help();
    }

    let action = args.remove(0).to_lowercase();

    match action.as_str() {
        "create" | "add" | "new" => {
            // cron create <prompt> <schedule> [--name <name>] [--skills <skill1,skill2>] [--repeat <n>]
            let (prompt, schedule) = if args.len() >= 2 {
                (args[0].clone(), args[1].clone())
            } else if args.len() == 1 {
                ("".to_string(), args[0].clone())
            } else {
                return Ok("Usage: cron create <prompt> <schedule>\nExample: cron create \"Run tests\" \"every 30m\"".to_string());
            };

            let mut name = None;
            let mut skills = vec![];
            let mut repeat = None;
            let mut deliver = "local".to_string();
            let mut i = 2;

            while i < args.len() {
                match args[i].as_str() {
                    "--name" | "-n" => {
                        if i + 1 < args.len() {
                            name = Some(args[i + 1].as_str());
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "--skills" | "-s" => {
                        if i + 1 < args.len() {
                            skills = args[i + 1].split(',').map(String::from).collect();
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "--repeat" | "-r" => {
                        if i + 1 < args.len() {
                            repeat = args[i + 1].parse().ok();
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "--deliver" | "-d" => {
                        if i + 1 < args.len() {
                            deliver = args[i + 1].clone();
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    _ => i += 1,
                }
            }

            let result = cron::create_job(&prompt, &schedule, name, repeat, skills, Some(&deliver), None, None, None)?;
            Ok(format_cron_output(&result))
        }
        "list" | "ls" | "l" => {
            let include_disabled = args.iter().any(|a| a == "--all" || a == "-a");
            let result = cron::list_jobs()?;
            let jobs: Vec<CronJobSummary> = result.jobs.unwrap_or_default();

            if !include_disabled {
                let jobs: Vec<CronJobSummary> = jobs.into_iter().filter(|j| j.enabled).collect();
                return Ok(cron::format_jobs_list(&jobs));
            }
            Ok(cron::format_jobs_list(&jobs))
        }
        "pause" => {
            if args.is_empty() {
                return Ok("Usage: cron pause <job_id> [--reason <reason>]".to_string());
            }
            let job_id = &args[0];
            let reason = args.get(2).map(|s| s.as_str());
            let result = cron::pause_job(job_id, reason)?;
            Ok(format_cron_output(&result))
        }
        "resume" => {
            if args.is_empty() {
                return Ok("Usage: cron resume <job_id>".to_string());
            }
            let result = cron::resume_job(&args[0])?;
            Ok(format_cron_output(&result))
        }
        "remove" | "delete" | "rm" => {
            if args.is_empty() {
                return Ok("Usage: cron remove <job_id>".to_string());
            }
            let result = cron::remove_job(&args[0])?;
            Ok(format_cron_output(&result))
        }
        "run" | "trigger" => {
            if args.is_empty() {
                return Ok("Usage: cron run <job_id>".to_string());
            }
            let result = cron::trigger_job(&args[0])?;
            Ok(format_cron_output(&result))
        }
        "update" => {
            if args.len() < 3 {
                return Ok("Usage: cron update <job_id> <field> <value>".to_string());
            }
            let job_id = &args[0];
            let field = &args[1];
            let value = &args[2];

            let mut updates = std::collections::HashMap::new();
            match field.as_str() {
                "prompt" | "name" | "schedule" | "deliver" => {
                    updates.insert(field.clone(), serde_json::json!(value));
                }
                _ => {
                    return Ok(format!("Unknown field: {}. Valid fields: prompt, name, schedule, deliver", field));
                }
            }

            let result = cron::update_job(job_id, updates)?;
            Ok(format_cron_output(&result))
        }
        "due" => {
            let jobs = cron::get_due_jobs()?;
            Ok(format!("# Due Jobs\n\nFound {} due job(s).\n", jobs.len()))
        }
        "help" => show_cron_help(),
        _ => {
            // Try as shorthand: cron <schedule> <prompt>
            let schedule = &action;
            let prompt = args.join(" ");
            if !prompt.is_empty() && !schedule.is_empty() {
                let result = cron::create_job(&prompt, schedule, None, None, vec![], Some("local"), None, None, None)?;
                return Ok(format_cron_output(&result));
            }
            show_cron_help()
        }
    }
}

fn show_cron_help() -> Result<String> {
    let output = r#"# Cron Scheduler

Schedule tasks to run automatically at specific times.

## Usage

```
cron create <prompt> <schedule>   Create a new scheduled job
cron list [--all]                 List scheduled jobs
cron pause <job_id>              Pause a job
cron resume <job_id>              Resume a paused job
cron remove <job_id>              Remove a job
cron run <job_id>                Trigger a job immediately
cron update <job_id> <field> <val>  Update job properties
cron due                          Show jobs due to run
```

## Schedule Formats

```
30m        In 30 minutes (one-shot)
2h         In 2 hours (one-shot)
every 30m  Every 30 minutes (recurring)
every 2h   Every 2 hours (recurring)
every 1d   Every day (recurring)
0 9 * * *  Cron expression (daily at 9am)
2026-02-03T14:00  At specific timestamp
```

## Examples

```
cron create "Run tests" "every 1h"
cron create "Check backups" "every 1d" --name "Backup Check"
cron create "Generate report" "0 9 * * *" --repeat 7
cron list
cron pause abc123
cron resume abc123
cron remove abc123
```

## Options

```
--name <name>      Friendly name for the job
--skills <skills>  Comma-separated skill names to load
--repeat <n>       Repeat N times (default: 1 for one-shot, infinite for recurring)
--deliver <target> Delivery target (local, origin)
```
"#.to_string();
    Ok(output)
}

fn format_cron_output(result: &CronJobOutput) -> String {
    if result.success {
        let mut output = String::new();
        if let Some(msg) = &result.message {
            output.push_str(&format!("{}\n", msg));
        }
        if let Some(id) = &result.job_id {
            output.push_str(&format!("Job ID: {}\n", id));
        }
        if let Some(schedule) = &result.schedule {
            output.push_str(&format!("Schedule: {}\n", schedule));
        }
        if let Some(next) = &result.next_run_at {
            output.push_str(&format!("Next run: {}\n", next));
        }
        output
    } else {
        format!("Error: {}", result.error.as_deref().unwrap_or("Unknown error"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_help() {
        let output = show_cron_help().unwrap();
        assert!(output.contains("Cron Scheduler"));
    }
}
