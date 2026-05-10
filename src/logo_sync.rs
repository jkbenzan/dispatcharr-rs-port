use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use tracing::{info, error};
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogoRepository {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct LogoSearchResult {
    pub name: String,
    pub path: String,
    pub repository: String,
}

pub async fn sync_logo_repositories(_state: Arc<AppState>) {
    info!("[LogoSync] Starting logo repository synchronization...");
    
    // For now, we'll use a hardcoded list or load from settings
    // In a future step, these could be user-configurable in the DB
    let repos = vec![
        LogoRepository {
            name: "tv-logos".to_string(),
            url: "https://github.com/tv-logo/tv-logos.git".to_string(),
            enabled: true,
        },
        LogoRepository {
            name: "iptv-org-logos".to_string(),
            url: "https://github.com/iptv-org/logos.git".to_string(),
            enabled: true,
        },
    ];

    let base_dir = Path::new("data/logo-libraries");
    if !base_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(base_dir) {
            error!("[LogoSync] Failed to create logo-libraries directory: {}", e);
            return;
        }
    }

    for repo in repos {
        if !repo.enabled { continue; }

        let repo_path = base_dir.join(&repo.name);
        if repo_path.exists() {
            info!("[LogoSync] Updating repository: {}", repo.name);
            let output = Command::new("git")
                .arg("-C")
                .arg(&repo_path)
                .arg("pull")
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    info!("[LogoSync] Successfully updated {}", repo.name);
                }
                Ok(out) => {
                    error!("[LogoSync] Failed to update {}: {}", repo.name, String::from_utf8_lossy(&out.stderr));
                }
                Err(e) => {
                    error!("[LogoSync] Failed to execute git pull for {}: {}", repo.name, e);
                }
            }
        } else {
            info!("[LogoSync] Cloning repository: {}", repo.name);
            let output = Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg(&repo.url)
                .arg(&repo_path)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    info!("[LogoSync] Successfully cloned {}", repo.name);
                }
                Ok(out) => {
                    error!("[LogoSync] Failed to clone {}: {}", repo.name, String::from_utf8_lossy(&out.stderr));
                }
                Err(e) => {
                    error!("[LogoSync] Failed to execute git clone for {}: {}", repo.name, e);
                }
            }
        }
    }
}

pub fn search_logo_libraries(query: &str) -> Vec<LogoSearchResult> {
    let mut results = Vec::new();
    let base_dir = Path::new("data/logo-libraries");
    if !base_dir.exists() { return results; }

    let query_lower = query.to_lowercase();

    for entry in WalkDir::new(base_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok()) {
        
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "png" || ext_str == "jpg" || ext_str == "jpeg" || ext_str == "svg" {
                    let filename = path.file_name().unwrap().to_string_lossy();
                    if filename.to_lowercase().contains(&query_lower) {
                        // Extract repo name from path
                        let repo_name = path.strip_prefix(base_dir)
                            .ok()
                            .and_then(|p| p.components().next())
                            .map(|c| c.as_os_str().to_string_lossy().to_string())
                            .unwrap_or_default();

                        results.push(LogoSearchResult {
                            name: filename.to_string(),
                            path: path.to_string_lossy().to_string(),
                            repository: repo_name,
                        });
                    }
                }
            }
        }

        if results.len() >= 100 { break; }
    }

    results
}
