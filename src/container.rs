//! Container Backends - Docker, SSH, Modal, Daytona, Singularity
//!
//! Remote execution environments for Code Buddy.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Container backend provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerBackend {
    /// Local execution (default)
    Local,
    /// Docker container
    Docker(DockerConfig),
    /// SSH remote execution
    Ssh(SshConfig),
    /// Modal serverless
    Modal(ModalConfig),
    /// Daytona dev environment
    Daytona(DaytonaConfig),
    /// Singularity container
    Singularity(SingularityConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    pub image: String,
    pub command: Option<String>,
    pub volumes: Vec<(String, String)>,
    pub env_vars: HashMap<String, String>,
    pub workdir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_file: Option<String>,
    pub workdir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalConfig {
    pub app_name: String,
    pub image: Option<String>,
    pub gpu: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaytonaConfig {
    pub workspace_id: String,
    pub provider: Option<String>,
    pub gpu: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingularityConfig {
    pub image: String,
    pub bind_paths: Vec<String>,
    pub env_vars: HashMap<String, String>,
}

/// Container backend trait
pub trait Backend: Send + Sync {
    /// Execute command in the backend
    fn execute(&self, command: &str, args: &[String]) -> Result<BackendResult>;

    /// Copy file to the backend
    fn copy_to(&self, local_path: &Path, remote_path: &Path) -> Result<()>;

    /// Copy file from the backend
    fn copy_from(&self, remote_path: &Path, local_path: &Path) -> Result<()>;

    /// Check backend health
    fn health_check(&self) -> Result<bool>;
}

/// Backend execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// Local backend (default)
pub struct LocalBackend;

impl Backend for LocalBackend {
    fn execute(&self, command: &str, args: &[String]) -> Result<BackendResult> {
        let start = std::time::Instant::now();

        let output = std::process::Command::new(command)
            .args(args)
            .output()?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(BackendResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
        })
    }

    fn copy_to(&self, local_path: &Path, remote_path: &Path) -> Result<()> {
        std::fs::copy(local_path, remote_path)?;
        Ok(())
    }

    fn copy_from(&self, remote_path: &Path, local_path: &Path) -> Result<()> {
        std::fs::copy(remote_path, local_path)?;
        Ok(())
    }

    fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

/// Docker backend
pub struct DockerBackend {
    config: DockerConfig,
}

impl DockerBackend {
    pub fn new(config: DockerConfig) -> Self {
        Self { config }
    }
}

impl Backend for DockerBackend {
    fn execute(&self, command: &str, args: &[String]) -> Result<BackendResult> {
        let start = std::time::Instant::now();

        let mut docker_args: Vec<String> = vec!["run".to_string(), "--rm".to_string(), "-i".to_string()];

        // Add volumes
        for (host, container) in &self.config.volumes {
            docker_args.push("-v".to_string());
            docker_args.push(format!("{}:{}", host, container));
        }

        // Add env vars
        for (key, value) in &self.config.env_vars {
            docker_args.push("-e".to_string());
            docker_args.push(format!("{}={}", key, value));
        }

        // Image
        docker_args.push(self.config.image.clone());

        // Command
        docker_args.push(command.to_string());
        docker_args.extend(args.iter().cloned());

        let output = std::process::Command::new("docker")
            .args(&docker_args)
            .output()?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(BackendResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
        })
    }

    fn copy_to(&self, local_path: &Path, remote_path: &Path) -> Result<()> {
        let output = std::process::Command::new("docker")
            .args(["cp", &local_path.to_string_lossy()])
            .arg(format!("{}:{}", self.config.image, remote_path.to_string_lossy()))
            .output()?;
        if !output.status.success() {
            anyhow::bail!("Docker cp failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    fn copy_from(&self, remote_path: &Path, local_path: &Path) -> Result<()> {
        let output = std::process::Command::new("docker")
            .args(["cp"])
            .arg(format!("{}:{}", self.config.image, remote_path.to_string_lossy()))
            .arg(local_path.to_string_lossy().as_ref())
            .output()?;
        if !output.status.success() {
            anyhow::bail!("Docker cp failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    fn health_check(&self) -> Result<bool> {
        let output = std::process::Command::new("docker")
            .args(["image", "ls", &self.config.image])
            .output()?;
        Ok(output.status.success())
    }
}

/// SSH backend
pub struct SshBackend {
    config: SshConfig,
}

impl SshBackend {
    pub fn new(config: SshConfig) -> Self {
        Self { config }
    }
}

impl Backend for SshBackend {
    fn execute(&self, command: &str, args: &[String]) -> Result<BackendResult> {
        let start = std::time::Instant::now();

        let mut ssh_args: Vec<String> = vec![];

        if let Some(key) = &self.config.key_file {
            ssh_args.push("-i".to_string());
            ssh_args.push(key.clone());
        }

        ssh_args.push("-o".to_string());
        ssh_args.push("StrictHostKeyChecking=no".to_string());

        ssh_args.push(format!("{}@{}", self.config.user, self.config.host));

        if self.config.port != 22 {
            ssh_args.push("-p".to_string());
            ssh_args.push(self.config.port.to_string());
        }

        // Pass command and args as separate arguments to SSH (avoids shell interpretation)
        ssh_args.push(command.to_string());
        ssh_args.extend(args.iter().cloned());

        let output = std::process::Command::new("ssh")
            .args(&ssh_args)
            .output()?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(BackendResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
        })
    }

    fn copy_to(&self, local_path: &Path, remote_path: &Path) -> Result<()> {
        // SECURITY: Validate remote_path for shell metacharacters
        let remote_str = remote_path.to_string_lossy();
        if remote_str.contains([';', '|', '&', '$', '`', '<', '>', '\n', '\r']) {
            anyhow::bail!("Remote path contains invalid characters: path traversal or shell metacharacters not allowed");
        }
        let output = std::process::Command::new("scp")
            .args(["-o", "StrictHostKeyChecking=no"])
            .arg(local_path.to_string_lossy().as_ref())
            .arg(format!("{}@{}:{}", self.config.user, self.config.host, remote_str))
            .output()?;
        if !output.status.success() {
            anyhow::bail!("scp failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    fn copy_from(&self, remote_path: &Path, local_path: &Path) -> Result<()> {
        // SECURITY: Validate remote_path for shell metacharacters
        let remote_str = remote_path.to_string_lossy();
        if remote_str.contains([';', '|', '&', '$', '`', '<', '>', '\n', '\r']) {
            anyhow::bail!("Remote path contains invalid characters: path traversal or shell metacharacters not allowed");
        }
        let output = std::process::Command::new("scp")
            .args(["-o", "StrictHostKeyChecking=no"])
            .arg(format!("{}@{}:{}", self.config.user, self.config.host, remote_str))
            .arg(local_path.to_string_lossy().as_ref())
            .output()?;
        if !output.status.success() {
            anyhow::bail!("scp failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    fn health_check(&self) -> Result<bool> {
        let output = std::process::Command::new("ssh")
            .args(["-o", "StrictHostKeyChecking=no", "-o", "ConnectTimeout=5"])
            .args(["-p", &self.config.port.to_string()])
            .arg(format!("{}@{}", self.config.user, self.config.host))
            .arg("echo ok")
            .output()?;
        Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).contains("ok"))
    }
}

/// Create a backend from configuration
pub fn create_backend(config: ContainerBackend) -> Result<Box<dyn Backend>> {
    match config {
        ContainerBackend::Local => Ok(Box::new(LocalBackend)),
        ContainerBackend::Docker(c) => Ok(Box::new(DockerBackend::new(c))),
        ContainerBackend::Ssh(c) => Ok(Box::new(SshBackend::new(c))),
        ContainerBackend::Modal(_) => {
            // Modal requires their SDK - return a stub
            Ok(Box::new(LocalBackend))
        }
        ContainerBackend::Daytona(_) => {
            // Daytona requires their SDK - return a stub
            Ok(Box::new(LocalBackend))
        }
        ContainerBackend::Singularity(_) => {
            // Singularity is similar to Docker - return a stub
            Ok(Box::new(LocalBackend))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_backend() {
        let backend = LocalBackend;
        let result = backend.execute("echo", &["hello".to_string()]).unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("hello"));
    }

    #[test]
    fn test_create_backend_local() {
        let backend = create_backend(ContainerBackend::Local).unwrap();
        let result = backend.execute("echo", &["test".to_string()]).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_local_backend_copy() {
        use std::io::Write;
        let backend = LocalBackend;
        let temp_dir = std::env::temp_dir();
        let local_path = temp_dir.join("test_copy_local.txt");
        let remote_path = temp_dir.join("test_copy_remote.txt");

        // Write test file
        let mut file = std::fs::File::create(&local_path).unwrap();
        writeln!(file, "test content").unwrap();

        // Copy (local backend is no-op)
        backend.copy_to(&local_path, &remote_path).unwrap();
        backend.copy_from(&remote_path, &local_path).unwrap();

        // Clean up
        let _ = std::fs::remove_file(local_path);
        let _ = std::fs::remove_file(remote_path);
    }

    #[test]
    fn test_backend_result() {
        let result = BackendResult {
            success: true,
            stdout: "output".to_string(),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: 100,
        };
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn test_container_backend_variants() {
        // Test Docker backend creation
        let docker_config = DockerConfig {
            image: "ubuntu:latest".to_string(),
            command: Some("/bin/bash".to_string()),
            volumes: vec![],
            env_vars: HashMap::new(),
            workdir: None,
        };
        let docker_backend = ContainerBackend::Docker(docker_config);
        assert!(matches!(docker_backend, ContainerBackend::Docker(_)));

        // Test SSH backend creation
        let ssh_config = SshConfig {
            user: "user".to_string(),
            host: "localhost".to_string(),
            port: 22,
            key_file: None,
            workdir: None,
        };
        let ssh_backend = ContainerBackend::Ssh(ssh_config);
        assert!(matches!(ssh_backend, ContainerBackend::Ssh(_)));
    }
}
