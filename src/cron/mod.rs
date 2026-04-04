//! Cron Scheduler - Built-in cron job management
//!
//! Provides scheduled task execution with:
//! - Duration parsing (30m, 2h, 1d)
//! - Interval scheduling (every 30m, every 2h)
//! - Cron expression support (0 9 * * *)
//! - One-shot timestamps (2026-02-03T14:00)
//! - Job persistence in JSON storage
//! - Output storage per job

use anyhow::Result;
use chrono::{DateTime, Duration, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// Schedule kind
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum Schedule {
    /// One-shot scheduled run
    Once { run_at: String },
    /// Recurring interval
    Interval { minutes: i64 },
    /// Cron expression
    Cron { expr: String },
}

impl Schedule {
    pub fn display(&self) -> String {
        match self {
            Schedule::Once { run_at } => format!("once at {}", run_at),
            Schedule::Interval { minutes } => {
                if *minutes >= 60 * 24 {
                    format!("every {}d", minutes / (60 * 24))
                } else if *minutes >= 60 {
                    format!("every {}h", minutes / 60)
                } else {
                    format!("every {}m", minutes)
                }
            }
            Schedule::Cron { expr } => expr.to_string(),
        }
    }
}

/// Cron job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub prompt: String,
    pub skills: Vec<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub schedule: Schedule,
    pub schedule_display: String,
    pub repeat_times: Option<i32>,
    pub repeat_completed: i32,
    pub enabled: bool,
    pub state: JobState,
    pub deliver: String,
    pub created_at: String,
    pub next_run_at: Option<String>,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    pub paused_at: Option<String>,
    pub paused_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum JobState {
    #[default]
    Scheduled,
    Paused,
    Running,
    Completed,
}

/// Output format for tool responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobOutput {
    pub success: bool,
    pub job_id: Option<String>,
    pub name: Option<String>,
    pub schedule: Option<String>,
    pub next_run_at: Option<String>,
    pub last_status: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub jobs: Option<Vec<CronJobSummary>>,
    pub count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobSummary {
    pub id: String,
    pub name: String,
    pub prompt_preview: String,
    pub skills: Vec<String>,
    pub schedule: String,
    pub next_run_at: Option<String>,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    pub state: String,
    pub enabled: bool,
}

/// Get the cron data directory
fn get_cron_dir() -> PathBuf {
    crate::dirs::code_buddy_home()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".code-buddy"))
        .join("cron")
}

/// Get the jobs file path
fn get_jobs_file() -> PathBuf {
    get_cron_dir().join("jobs.json")
}

/// Get the output directory for a job
fn get_output_dir(job_id: &str) -> PathBuf {
    get_cron_dir().join("output").join(job_id)
}

/// Load all jobs from storage
pub fn load_jobs() -> Result<Vec<CronJob>> {
    let cron_dir = get_cron_dir();
    let jobs_file = get_jobs_file();

    if !cron_dir.exists() {
        fs::create_dir_all(&cron_dir)?;
    }

    if !jobs_file.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&jobs_file)?;
    let data: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
    let jobs = data.get("jobs").and_then(|j| j.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|j| serde_json::from_value(j.clone()).ok())
            .collect()
    }).unwrap_or_default();

    Ok(jobs)
}

/// Save all jobs to storage
fn save_jobs(jobs: &[CronJob]) -> Result<()> {
    let cron_dir = get_cron_dir();
    fs::create_dir_all(&cron_dir)?;

    let data = serde_json::json!({
        "jobs": jobs,
        "updated_at": Utc::now().to_rfc3339()
    });

    let jobs_file = get_jobs_file();
    let temp_file = cron_dir.join(".jobs.tmp");

    fs::write(&temp_file, serde_json::to_string_pretty(&data)?)?;
    fs::rename(&temp_file, &jobs_file)?;

    Ok(())
}

/// Parse a schedule string into Schedule
pub fn parse_schedule(input: &str) -> Result<Schedule> {
    let input = input.trim().to_lowercase();

    // "every X" pattern → recurring interval
    if input.starts_with("every ") {
        let duration_str = input.trim_start_matches("every ").trim();
        let minutes = parse_duration(duration_str)?;
        return Ok(Schedule::Interval { minutes });
    }

    // Cron expression (5 fields)
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() >= 5 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit() || c == '*' || c == '-' || c == ',' || c == '/')) {
        // Basic cron validation
        let expr = input.split_whitespace().take(5).collect::<Vec<_>>().join(" ");
        return Ok(Schedule::Cron { expr });
    }

    // ISO timestamp
    if input.contains('T') || input.starts_with(|c: char| c.is_ascii_digit()) {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&input) {
            return Ok(Schedule::Once { run_at: dt.to_rfc3339() });
        }
        // Try parsing without timezone
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&input, "%Y-%m-%dT%H:%M:%S") {
            let local = dt.and_local_timezone(Local).single();
            if let Some(local) = local {
                return Ok(Schedule::Once { run_at: local.to_rfc3339() });
            }
        }
    }

    // Duration like "30m", "2h", "1d"
    let minutes = parse_duration(&input)?;
    let run_at = Local::now() + Duration::minutes(minutes);
    Ok(Schedule::Once { run_at: run_at.to_rfc3339() })
}

/// Parse duration string into minutes
fn parse_duration(s: &str) -> Result<i64> {
    let s = s.trim().to_lowercase();
    let re = regex::Regex::new(r"^(\d+)\s*(m|min|mins|h|hr|hrs|d|day|days)$")?;
    if let Some(caps) = re.captures(&s) {
        let value: i64 = caps.get(1).and_then(|m| m.as_str().parse().ok()).ok_or_else(|| anyhow::anyhow!("Invalid duration value in '{}'", s))?;
        let unit = caps.get(2).and_then(|m| m.as_str().parse::<char>().ok()).ok_or_else(|| anyhow::anyhow!("Invalid duration unit in '{}'", s))?;
        let multiplier = match unit {
            'm' => 1,
            'h' => 60,
            'd' => 60 * 24,
            _ => 1,
        };
        return Ok(value * multiplier);
    }
    anyhow::bail!("Invalid duration: '{}'. Use format like '30m', '2h', or '1d'", s)
}

/// Compute next run time
fn compute_next_run(schedule: &Schedule, last_run_at: Option<&str>) -> Option<String> {
    let now = Local::now();

    match schedule {
        Schedule::Once { run_at } => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(run_at) {
                let local_dt: DateTime<Local> = dt.into();
                if local_dt > now {
                    return Some(local_dt.to_rfc3339());
                }
            }
            None
        }
        Schedule::Interval { minutes } => {
            let next = if let Some(last) = last_run_at {
                if let Ok(dt) = DateTime::parse_from_rfc3339(last) {
                    let local: DateTime<Local> = dt.into();
                    local + Duration::minutes(*minutes)
                } else {
                    now + Duration::minutes(*minutes)
                }
            } else {
                now + Duration::minutes(*minutes)
            };
            Some(next.to_rfc3339())
        }
        Schedule::Cron { expr: _ } => {
            // For cron, we'd need the croniter crate
            // For now, return a simple approximation
            Some((now + Duration::hours(1)).to_rfc3339())
        }
    }
}

/// Get current timestamp
fn now_iso() -> String {
    Local::now().to_rfc3339()
}

/// Create a new cron job
#[allow(clippy::too_many_arguments)]
pub fn create_job(
    prompt: &str,
    schedule_str: &str,
    name: Option<&str>,
    repeat: Option<i32>,
    skills: Vec<String>,
    deliver: Option<&str>,
    model: Option<&str>,
    provider: Option<&str>,
    base_url: Option<&str>,
) -> Result<CronJobOutput> {
    let schedule = parse_schedule(schedule_str)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let id = Uuid::new_v4().to_string()[..12].to_string();
    let default_name = {
        let label = if !prompt.is_empty() { prompt } else if !skills.is_empty() { &skills[0] } else { "cron job" };
        label.chars().take(50).collect::<String>()
    };
    let name: String = name.map(String::from).unwrap_or(default_name);

    let is_once = matches!(schedule, Schedule::Once { .. });
    let repeat_times = if is_once {
        Some(1)
    } else {
        repeat
    };

    let next_run_at = compute_next_run(&schedule, None);

    let job = CronJob {
        id: id.clone(),
        name: name.clone(),
        prompt: prompt.to_string(),
        skills: skills.clone(),
        model: model.map(String::from),
        provider: provider.map(String::from),
        base_url: base_url.map(String::from),
        schedule: schedule.clone(),
        schedule_display: schedule.display(),
        repeat_times,
        repeat_completed: 0,
        enabled: true,
        state: JobState::Scheduled,
        deliver: deliver.unwrap_or("local").to_string(),
        created_at: now_iso(),
        next_run_at: next_run_at.clone(),
        last_run_at: None,
        last_status: None,
        last_error: None,
        paused_at: None,
        paused_reason: None,
    };

    let mut jobs = load_jobs()?;
    jobs.push(job.clone());
    save_jobs(&jobs)?;

    Ok(CronJobOutput {
        success: true,
        job_id: Some(id),
        name: Some(name),
        schedule: Some(schedule.display()),
        next_run_at,
        last_status: None,
        message: Some(format!("Cron job '{}' created.", job.name)),
        error: None,
        jobs: None,
        count: None,
    })
}

/// List all jobs
pub fn list_jobs() -> Result<CronJobOutput> {
    let jobs = load_jobs()?;
    let count = jobs.len();
    let summaries: Vec<CronJobSummary> = jobs.iter().map(|j| CronJobSummary {
        id: j.id.clone(),
        name: j.name.clone(),
        prompt_preview: if j.prompt.len() > 100 {
            format!("{}...", &j.prompt[..100])
        } else {
            j.prompt.clone()
        },
        skills: j.skills.clone(),
        schedule: j.schedule_display.clone(),
        next_run_at: j.next_run_at.clone(),
        last_run_at: j.last_run_at.clone(),
        last_status: j.last_status.clone(),
        state: format!("{:?}", j.state),
        enabled: j.enabled,
    }).collect();

    Ok(CronJobOutput {
        success: true,
        job_id: None,
        name: None,
        schedule: None,
        next_run_at: None,
        last_status: None,
        message: None,
        error: None,
        jobs: Some(summaries),
        count: Some(count),
    })
}

/// Get a job by ID
pub fn get_job(job_id: &str) -> Result<Option<CronJob>> {
    let jobs = load_jobs()?;
    Ok(jobs.into_iter().find(|j| j.id == job_id))
}

/// Update a job
pub fn update_job(job_id: &str, updates: HashMap<String, serde_json::Value>) -> Result<CronJobOutput> {
    let mut jobs = load_jobs()?;
    let mut found = false;
    let mut updated_job: Option<CronJob> = None;

    for job in jobs.iter_mut() {
        if job.id == job_id {
            found = true;

            if let Some(name) = updates.get("name").and_then(|v| v.as_str()) {
                job.name = name.to_string();
            }
            if let Some(prompt) = updates.get("prompt").and_then(|v| v.as_str()) {
                job.prompt = prompt.to_string();
            }
            if let Some(skills_val) = updates.get("skills").and_then(|v| v.as_array()) {
                job.skills = skills_val.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            }
            if let Some(deliver) = updates.get("deliver").and_then(|v| v.as_str()) {
                job.deliver = deliver.to_string();
            }
            if let Some(schedule_str) = updates.get("schedule").and_then(|v| v.as_str()) {
                job.schedule = parse_schedule(schedule_str)?;
                job.schedule_display = job.schedule.display();
            }

            job.next_run_at = compute_next_run(&job.schedule, job.last_run_at.as_deref());
            if job.state != JobState::Paused {
                job.state = JobState::Scheduled;
                job.enabled = true;
            }

            updated_job = Some(job.clone());
        }
    }

    if !found {
        return Ok(CronJobOutput {
            success: false,
            job_id: None,
            name: None,
            schedule: None,
            next_run_at: None,
            last_status: None,
            message: None,
            error: Some(format!("Job '{}' not found", job_id)),
            jobs: None,
            count: None,
        });
    }

    save_jobs(&jobs)?;

    Ok(CronJobOutput {
        success: true,
        job_id: Some(job_id.to_string()),
        name: updated_job.as_ref().map(|j| j.name.clone()),
        schedule: updated_job.as_ref().map(|j| j.schedule_display.clone()),
        next_run_at: updated_job.as_ref().and_then(|j| j.next_run_at.clone()),
        last_status: None,
        message: Some(format!("Job '{}' updated.", job_id)),
        error: None,
        jobs: None,
        count: None,
    })
}

/// Pause a job
pub fn pause_job(job_id: &str, reason: Option<&str>) -> Result<CronJobOutput> {
    let mut jobs = load_jobs()?;
    let mut found_job: Option<CronJob> = None;

    for job in jobs.iter_mut() {
        if job.id == job_id {
            job.enabled = false;
            job.state = JobState::Paused;
            job.paused_at = Some(now_iso());
            job.paused_reason = reason.map(String::from);
            found_job = Some(job.clone());
            break;
        }
    }

    save_jobs(&jobs)?;

    if let Some(job) = found_job {
        Ok(CronJobOutput {
            success: true,
            job_id: Some(job.id),
            name: Some(job.name),
            schedule: Some(job.schedule_display),
            next_run_at: job.next_run_at,
            last_status: job.last_status,
            message: Some("Job paused.".to_string()),
            error: None,
            jobs: None,
            count: None,
        })
    } else {
        Ok(CronJobOutput {
            success: false,
            job_id: None,
            name: None,
            schedule: None,
            next_run_at: None,
            last_status: None,
            message: None,
            error: Some(format!("Job '{}' not found", job_id)),
            jobs: None,
            count: None,
        })
    }
}

/// Resume a paused job
pub fn resume_job(job_id: &str) -> Result<CronJobOutput> {
    let mut jobs = load_jobs()?;
    let mut found_job: Option<CronJob> = None;

    for job in jobs.iter_mut() {
        if job.id == job_id {
            job.enabled = true;
            job.state = JobState::Scheduled;
            job.paused_at = None;
            job.paused_reason = None;
            job.next_run_at = compute_next_run(&job.schedule, job.last_run_at.as_deref());
            found_job = Some(job.clone());
            break;
        }
    }

    save_jobs(&jobs)?;

    if let Some(job) = found_job {
        Ok(CronJobOutput {
            success: true,
            job_id: Some(job.id),
            name: Some(job.name),
            schedule: Some(job.schedule_display),
            next_run_at: job.next_run_at,
            last_status: job.last_status,
            message: Some("Job resumed.".to_string()),
            error: None,
            jobs: None,
            count: None,
        })
    } else {
        Ok(CronJobOutput {
            success: false,
            job_id: None,
            name: None,
            schedule: None,
            next_run_at: None,
            last_status: None,
            message: None,
            error: Some(format!("Job '{}' not found", job_id)),
            jobs: None,
            count: None,
        })
    }
}

/// Remove a job
pub fn remove_job(job_id: &str) -> Result<CronJobOutput> {
    let mut jobs = load_jobs()?;
    let initial_len = jobs.len();
    jobs.retain(|j| j.id != job_id);

    if jobs.len() < initial_len {
        save_jobs(&jobs)?;
        Ok(CronJobOutput {
            success: true,
            job_id: Some(job_id.to_string()),
            name: None,
            schedule: None,
            next_run_at: None,
            last_status: None,
            message: Some(format!("Job '{}' removed.", job_id)),
            error: None,
            jobs: None,
            count: None,
        })
    } else {
        Ok(CronJobOutput {
            success: false,
            job_id: None,
            name: None,
            schedule: None,
            next_run_at: None,
            last_status: None,
            message: None,
            error: Some(format!("Job '{}' not found", job_id)),
            jobs: None,
            count: None,
        })
    }
}

/// Trigger a job to run immediately
pub fn trigger_job(job_id: &str) -> Result<CronJobOutput> {
    let mut jobs = load_jobs()?;
    let mut found_job: Option<CronJob> = None;

    for job in jobs.iter_mut() {
        if job.id == job_id {
            job.enabled = true;
            job.state = JobState::Scheduled;
            job.paused_at = None;
            job.paused_reason = None;
            job.next_run_at = Some(now_iso());
            found_job = Some(job.clone());
            break;
        }
    }

    save_jobs(&jobs)?;

    if let Some(job) = found_job {
        Ok(CronJobOutput {
            success: true,
            job_id: Some(job.id),
            name: Some(job.name),
            schedule: Some(job.schedule_display),
            next_run_at: job.next_run_at,
            last_status: job.last_status,
            message: Some("Job triggered.".to_string()),
            error: None,
            jobs: None,
            count: None,
        })
    } else {
        Ok(CronJobOutput {
            success: false,
            job_id: None,
            name: None,
            schedule: None,
            next_run_at: None,
            last_status: None,
            message: None,
            error: Some(format!("Job '{}' not found", job_id)),
            jobs: None,
            count: None,
        })
    }
}

/// Mark a job as run and compute next run
pub fn mark_job_run(job_id: &str, success: bool, error: Option<&str>) -> Result<()> {
    let mut jobs = load_jobs()?;

    for job in jobs.iter_mut() {
        if job.id == job_id {
            job.last_run_at = Some(now_iso());
            job.last_status = Some(if success { "ok".to_string() } else { "error".to_string() });
            job.last_error = error.map(String::from);
            job.repeat_completed += 1;

            // Check repeat limit
            if let Some(limit) = job.repeat_times {
                if job.repeat_completed >= limit {
                    // Remove completed job
                    jobs.retain(|j| j.id != job_id);
                    save_jobs(&jobs)?;
                    return Ok(());
                }
            }

            // Compute next run
            job.next_run_at = compute_next_run(&job.schedule, job.last_run_at.as_deref());
            if job.next_run_at.is_none() {
                job.enabled = false;
                job.state = JobState::Completed;
            } else if job.state != JobState::Paused {
                job.state = JobState::Scheduled;
            }

            save_jobs(&jobs)?;
            return Ok(());
        }
    }

    Ok(())
}

/// Get all due jobs
pub fn get_due_jobs() -> Result<Vec<CronJob>> {
    let now = Local::now();
    let jobs = load_jobs()?;
    let now_ts = now.timestamp();

    let due: Vec<CronJob> = jobs.into_iter().filter(|job| {
        if !job.enabled {
            return false;
        }
        if let Some(next_run) = &job.next_run_at {
            if let Ok(dt) = DateTime::parse_from_rfc3339(next_run) {
                let local: DateTime<Local> = dt.into();
                return local.timestamp() <= now_ts;
            }
        }
        false
    }).collect();

    Ok(due)
}

/// Save job output to file
pub fn save_job_output(job_id: &str, output: &str) -> Result<PathBuf> {
    let output_dir = get_output_dir(job_id);
    fs::create_dir_all(&output_dir)?;

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let output_file = output_dir.join(format!("{}.md", timestamp));

    fs::write(&output_file, output)?;

    Ok(output_file)
}

/// Format jobs list as markdown
pub fn format_jobs_list(jobs: &[CronJobSummary]) -> String {
    let mut md = String::from("# Scheduled Jobs\n\n");

    if jobs.is_empty() {
        md.push_str("No scheduled jobs. Use `/cron create <prompt> <schedule>` to create one.\n");
        return md;
    }

    for job in jobs {
        md.push_str(&format!("## {} ({})\n\n", job.name, job.id));
        md.push_str(&format!("- **Schedule:** {}\n", job.schedule));
        if let Some(next) = &job.next_run_at {
            md.push_str(&format!("- **Next Run:** {}\n", next));
        }
        if let Some(last) = &job.last_run_at {
            md.push_str(&format!("- **Last Run:** {} - **{}**\n", last, job.last_status.as_deref().unwrap_or("unknown")));
        }
        md.push_str(&format!("- **Status:** {:?}\n", job.state));
        if !job.skills.is_empty() {
            md.push_str(&format!("- **Skills:** {}\n", job.skills.join(", ")));
        }
        md.push_str(&format!("- **Enabled:** {}\n", job.enabled));
        md.push('\n');
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn setup_test_env() {
        // Set CODE_BUDDY_HOME to a unique temp directory for each test run
        let temp_dir = std::env::temp_dir().join(format!("test-cron-{}", nanoid::nanoid!(8)));
        std::fs::create_dir_all(&temp_dir).ok();
        std::env::set_var("CODE_BUDDY_HOME", temp_dir.to_string_lossy().to_string());
    }

    #[test]
    #[serial]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30m").unwrap(), 30);
        assert_eq!(parse_duration("2h").unwrap(), 120);
        assert_eq!(parse_duration("1d").unwrap(), 1440);
        assert_eq!(parse_duration("1h").unwrap(), 60);
    }

    #[test]
    #[serial]
    fn test_parse_schedule() {
        let s = parse_schedule("30m").unwrap();
        assert!(matches!(s, Schedule::Once { .. }));

        let s = parse_schedule("every 1h").unwrap();
        assert!(matches!(s, Schedule::Interval { .. }));

        let s = parse_schedule("0 9 * * *").unwrap();
        assert!(matches!(s, Schedule::Cron { .. }));
    }

    #[test]
    #[serial]
    fn test_create_and_list_job() {
        setup_test_env();
        let result = create_job(
            "Run tests",
            "30m",
            Some("Test Job"),
            None,
            vec![],
            Some("local"),
            None,
            None,
            None,
        ).unwrap();
        assert!(result.success);
        assert!(result.job_id.is_some());

        let list = list_jobs().unwrap();
        assert!(list.success);
        assert!(list.count.unwrap() >= 1);
    }

    #[test]
    #[serial]
    fn test_pause_and_resume_job() {
        setup_test_env();
        // Create a job first
        let create_result = create_job(
            "Pause test job",
            "1h",
            None,
            None,
            vec![],
            Some("local"),
            None,
            None,
            None,
        ).unwrap();
        let job_id = create_result.job_id.unwrap();

        // Pause the job
        let pause_result = pause_job(&job_id, Some("Testing pause")).unwrap();
        assert!(pause_result.success);

        // Resume the job
        let resume_result = resume_job(&job_id).unwrap();
        assert!(resume_result.success);

        // Try to pause non-existent job
        let result = pause_job("non-existent-id", None).unwrap();
        assert!(!result.success);
    }

    #[test]
    #[serial]
    fn test_remove_job() {
        setup_test_env();
        // Create a job first
        let create_result = create_job(
            "Remove test job",
            "2h",
            None,
            None,
            vec![],
            Some("local"),
            None,
            None,
            None,
        ).unwrap();
        let job_id = create_result.job_id.unwrap();

        // Remove the job
        let remove_result = remove_job(&job_id).unwrap();
        assert!(remove_result.success);

        // Try to remove non-existent job
        let result = remove_job("non-existent-id").unwrap();
        assert!(!result.success);
    }

    #[test]
    #[serial]
    fn test_trigger_job() {
        setup_test_env();
        // Create a job first
        let create_result = create_job(
            "Trigger test job",
            "3h",
            None,
            None,
            vec![],
            Some("local"),
            None,
            None,
            None,
        ).unwrap();
        let job_id = create_result.job_id.unwrap();

        // Trigger the job
        let trigger_result = trigger_job(&job_id).unwrap();
        assert!(trigger_result.success);

        // Try to trigger non-existent job
        let result = trigger_job("non-existent-id").unwrap();
        assert!(!result.success);
    }

    #[test]
    #[serial]
    fn test_mark_job_run() {
        setup_test_env();
        // Create a job first
        let create_result = create_job(
            "Mark test job",
            "4h",
            None,
            None,
            vec![],
            Some("local"),
            None,
            None,
            None,
        ).unwrap();
        let job_id = create_result.job_id.unwrap();

        // Mark as run successfully
        mark_job_run(&job_id, true, None).unwrap();

        // Mark as failed with error
        mark_job_run(&job_id, false, Some("Test error")).unwrap();

        // Mark non-existent job (should not panic)
        mark_job_run("non-existent-id", true, None).unwrap();
    }

    #[test]
    #[serial]
    fn test_get_due_jobs() {
        setup_test_env();
        // Create a job
        let create_result = create_job(
            "Due job test",
            "1h",
            None,
            None,
            vec![],
            Some("local"),
            None,
            None,
            None,
        ).unwrap();
        let job_id = create_result.job_id.unwrap();

        // Trigger it immediately
        trigger_job(&job_id).unwrap();

        // Get due jobs
        let due_jobs = get_due_jobs().unwrap();
        assert!(!due_jobs.is_empty());

        // Clean up
        remove_job(&job_id).unwrap();
    }

    #[test]
    fn test_format_jobs_list() {
        let summaries = vec![
            CronJobSummary {
                id: "test-id".to_string(),
                name: "Test Job".to_string(),
                prompt_preview: "Test prompt preview".to_string(),
                skills: vec![],
                schedule: "30m".to_string(),
                next_run_at: Some("2026-04-03T12:00:00Z".to_string()),
                last_run_at: None,
                last_status: None,
                state: "Scheduled".to_string(),
                enabled: true,
            }
        ];
        let output = format_jobs_list(&summaries);
        assert!(output.contains("Test Job"));
    }

    #[test]
    fn test_parse_invalid_duration() {
        assert!(parse_duration("invalid").is_err());
        assert!(parse_duration("").is_err());
        assert!(parse_duration("1x").is_err());
    }

    #[test]
    fn test_parse_invalid_schedule() {
        assert!(parse_schedule("").is_err());
        assert!(parse_schedule("invalid schedule").is_err());
    }
}
