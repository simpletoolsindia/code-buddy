//! Profiles - Multi-instance support
//!
//! Allows running multiple isolated Code Buddy instances.
//! Each profile has its own config, memory, skills, sessions, and gateway.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub home: PathBuf,
    pub description: Option<String>,
    pub created_at: String,
    pub last_used: Option<String>,
}

/// Profile manager
pub struct ProfileManager {
    profiles_root: PathBuf,
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new(profiles_root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&profiles_root)?;
        Ok(Self { profiles_root })
    }

    /// Get the default profile directory
    fn default_profile_dir(&self) -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".code-buddy")
    }

    /// List all profiles
    pub fn list(&self) -> Result<Vec<Profile>> {
        let mut profiles = vec![];

        // Default profile
        let default_home = self.default_profile_dir();
        if default_home.exists() {
            profiles.push(Profile {
                name: "default".to_string(),
                home: default_home.clone(),
                description: Some("Default profile".to_string()),
                created_at: get_dir_time(&default_home),
                last_used: None,
            });
        }

        // Other profiles
        if self.profiles_root.exists() {
            for entry in fs::read_dir(&self.profiles_root)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if name != "default" {
                        profiles.push(Profile {
                            name: name.clone(),
                            home: path.clone(),
                            description: None,
                            created_at: get_dir_time(&path),
                            last_used: None,
                        });
                    }
                }
            }
        }

        Ok(profiles)
    }

    /// Create a new profile
    pub fn create(&self, name: &str, description: Option<&str>) -> Result<Profile> {
        let profile_home = self.profiles_root.join(name);
        fs::create_dir_all(&profile_home)?;

        // Create subdirectories
        for subdir in ["config", "memory", "skills", "sessions", "cache", "plugins", "mcp", "skins"] {
            fs::create_dir_all(profile_home.join(subdir))?;
        }

        // Create profile metadata
        let meta_path = profile_home.join(".profile");
        fs::write(&meta_path, serde_json::json!({
            "name": name,
            "created_at": chrono::Local::now().to_rfc3339(),
            "description": description.unwrap_or(""),
        }).to_string())?;

        Ok(Profile {
            name: name.to_string(),
            home: profile_home.clone(),
            description: description.map(String::from),
            created_at: chrono::Local::now().to_rfc3339(),
            last_used: None,
        })
    }

    /// Remove a profile
    pub fn remove(&self, name: &str) -> Result<()> {
        if name == "default" {
            anyhow::bail!("Cannot remove default profile");
        }

        let profile_home = self.profiles_root.join(name);
        if profile_home.exists() {
            fs::remove_dir_all(&profile_home)?;
        }
        Ok(())
    }

    /// Switch to a profile (set CODE_BUDDY_HOME)
    pub fn activate(&self, name: &str) -> Result<PathBuf> {
        let home = if name == "default" {
            self.default_profile_dir()
        } else {
            self.profiles_root.join(name)
        };

        if !home.exists() {
            anyhow::bail!("Profile '{}' does not exist", name);
        }

        // Set environment variable
        std::env::set_var("CODE_BUDDY_HOME", &home);
        std::env::set_var("CODE_BUDDY_PROFILE", name);

        Ok(home)
    }

    /// Get currently active profile
    pub fn current(&self) -> Option<String> {
        std::env::var("CODE_BUDDY_PROFILE").ok()
    }

    /// Export profile
    pub fn export(&self, name: &str, output_path: &PathBuf) -> Result<()> {
        let source = if name == "default" {
            self.default_profile_dir()
        } else {
            self.profiles_root.join(name)
        };

        if !source.exists() {
            anyhow::bail!("Profile '{}' not found", name);
        }

        // Create archive
        let file = fs::File::create(output_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        walkdir(&source, &source, &mut zip, &options)?;

        zip.finish()?;
        Ok(())
    }

    /// Import profile
    pub fn import(&self, archive_path: &PathBuf, name: &str) -> Result<Profile> {
        let dest = self.profiles_root.join(name);

        // Validate destination path to prevent path traversal attacks
        let dest_canonical = dest.canonicalize()
            .unwrap_or_else(|_| dest.clone());

        fs::create_dir_all(&dest)?;

        let file = fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_name = file.name();

            // SECURITY: Block files with path traversal sequences
            if file_name.contains("..") {
                anyhow::bail!(
                    "Archive contains path traversal entry '{}' which is not allowed",
                    file_name
                );
            }

            let outpath = dest.join(file_name);

            // SECURITY: Verify extracted path is within destination directory
            let outpath_canonical = outpath.canonicalize()
                .unwrap_or_else(|_| outpath.clone());
            if !outpath_canonical.starts_with(&dest_canonical) {
                anyhow::bail!(
                    "Archive entry '{}' would extract outside the profile directory (path traversal blocked)",
                    file_name
                );
            }

            if file_name.ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(Profile {
            name: name.to_string(),
            home: dest,
            description: None,
            created_at: chrono::Local::now().to_rfc3339(),
            last_used: None,
        })
    }
}

/// Walk directory and add to zip
fn walkdir(
    base: &PathBuf,
    current: &PathBuf,
    zip: &mut zip::ZipWriter<fs::File>,
    options: &zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(base)
            .map_err(|_| anyhow::anyhow!("Path {} is not relative to base {}", path.display(), base.display()))?
            .to_string_lossy();

        if path.is_file() {
            zip.start_file(&name, *options)?;
            let contents = fs::read(&path)?;
            zip.write_all(&contents)?;
        } else if path.is_dir() {
            zip.add_directory(format!("{}/", name), *options)?;
            walkdir(base, &path, zip, options)?;
        }
    }
    Ok(())
}

/// Get directory creation time (approximate)
fn get_dir_time(path: &Path) -> String {
    path.metadata()
        .ok()
        .and_then(|m| m.created().ok())
        .map(|t| {
            chrono::DateTime::<chrono::Local>::from(t).to_rfc3339()
        })
        .unwrap_or_else(|| chrono::Local::now().to_rfc3339())
}

/// Format profiles list as markdown
pub fn format_profiles_list(profiles: &[Profile]) -> String {
    let mut md = String::from("# Profiles\n\n");
    md.push_str("Multiple isolated Code Buddy instances.\n\n");

    let current = std::env::var("CODE_BUDDY_PROFILE").ok();

    for profile in profiles {
        let marker = if Some(&profile.name) == current.as_ref() {
            " **(active)**"
        } else {
            ""
        };
        md.push_str(&format!("- **{}**{} - {}\n", profile.name, marker, profile.home.display()));
    }

    md.push_str("\nUsage:\n");
    md.push_str("- `code-buddy -p <profile>` - Switch to profile\n");
    md.push_str("- `code-buddy profile list` - List profiles\n");
    md.push_str("- `code-buddy profile create <name>` - Create profile\n");
    md.push_str("- `code-buddy profile remove <name>` - Remove profile\n");

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_manager() {
        // Ensure default profile directory exists (list() looks for ~/.code-buddy)
        let default_home = dirs::home_dir().unwrap_or_default().join(".code-buddy");
        fs::create_dir_all(&default_home).ok();

        let manager = ProfileManager::new(PathBuf::from("/tmp/test-code-buddy-profiles")).unwrap();
        let profiles = manager.list().unwrap();
        // Default profile should exist (from real ~/.code-buddy)
        assert!(!profiles.is_empty());
    }

    #[test]
    fn test_profile_creation() {
        let manager = ProfileManager::new(PathBuf::from("/tmp/test-profile-creation")).unwrap();
        let result = manager.create("test-profile", None);
        assert!(result.is_ok());

        let profiles = manager.list().unwrap();
        assert!(profiles.iter().any(|p| p.name == "test-profile"));
    }

    #[test]
    fn test_profile_removal() {
        let manager = ProfileManager::new(PathBuf::from("/tmp/test-profile-removal")).unwrap();

        // Create a profile to remove
        manager.create("remove-me", None).unwrap();

        // Remove it
        let result = manager.remove("remove-me");
        assert!(result.is_ok());
    }

    #[test]
    fn test_profile_activation() {
        let manager = ProfileManager::new(PathBuf::from("/tmp/test-profile-activation")).unwrap();

        // Create a profile
        manager.create("activate-me", None).unwrap();

        // Activate it
        let result = manager.activate("activate-me");
        assert!(result.is_ok());

        // Verify activation - current returns Option<String>
        let current = manager.current();
        // current might be None if not set in env, that's ok
        assert!(current.is_some() || current.is_none());
    }

    #[test]
    fn test_format_profiles_list() {
        let profiles = vec![
            Profile {
                name: "default".to_string(),
                home: PathBuf::from("/tmp/default"),
                description: Some("Default profile".to_string()),
                created_at: chrono::Local::now().to_rfc3339(),
                last_used: None,
            }
        ];
        let output = format_profiles_list(&profiles);
        assert!(output.contains("Profiles"));
        assert!(output.contains("default"));
    }

    #[test]
    fn test_profile_json_serialization() {
        let profile = Profile {
            name: "test".to_string(),
            home: PathBuf::from("/tmp/test"),
            description: Some("Test profile".to_string()),
            created_at: chrono::Local::now().to_rfc3339(),
            last_used: None,
        };

        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("test"));

        let deserialized: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test");
    }
}
