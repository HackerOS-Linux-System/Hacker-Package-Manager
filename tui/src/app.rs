use anyhow::{Context, Result};
use tokio::process::Command as AsyncCommand;

#[derive(Clone)]
pub struct Package {
    pub name: String,
    pub source: Source,
    pub description: String,
}

#[derive(Clone, PartialEq)]
pub enum Source {
    Apt,
    Snap,
    Flatpak,
    All,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::Apt => "APT",
            Source::Snap => "SNAP",
            Source::Flatpak => "FLATPAK",
            Source::All => "ALL",
        }
    }
}

pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub input: String,
    pub input_mode: InputMode,
    pub packages: Vec<Package>,
    pub package_list_state: ratatui::widgets::ListState,
    pub selected_source: Source,
    pub message: String,
    pub dot_count: usize,
}

impl App {
    pub fn new() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            packages: Vec::new(),
            package_list_state: ratatui::widgets::ListState::default(),
            selected_source: Source::All,
            message: String::new(),
            dot_count: 0,
        }
    }

    pub fn get_filtered_packages(&self) -> Vec<Package> {
        if self.selected_source == Source::All {
            self.packages.clone()
        } else {
            self.packages
                .iter()
                .filter(|p| p.source == self.selected_source)
                .cloned()
                .collect()
        }
    }
}

pub async fn search_packages(input: String) -> Result<Vec<Package>> {
    let mut packages = Vec::new();
    // Search APT
    let apt_output = AsyncCommand::new("apt-cache")
        .arg("search")
        .arg("--names-only")
        .arg(&input)
        .output()
        .await
        .context("Failed to execute apt-cache search")?;
    if apt_output.status.success() {
        let apt_str = String::from_utf8_lossy(&apt_output.stdout);
        for line in apt_str.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                if let Some((name, desc)) = trimmed.split_once(" - ") {
                    packages.push(Package {
                        name: name.trim().to_string(),
                        source: Source::Apt,
                        description: desc.trim().to_string(),
                    });
                }
            }
        }
    }
    // Search Snap
    let snap_output = AsyncCommand::new("snap")
        .arg("find")
        .arg(&input)
        .output()
        .await
        .context("Failed to execute snap find")?;
    if snap_output.status.success() {
        let snap_str = String::from_utf8_lossy(&snap_output.stdout);
        let lines: Vec<&str> = snap_str.lines().collect();
        let start = if !lines.is_empty() && lines[0].contains("Name") { 1 } else { 0 };
        for line in &lines[start..] {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 5 {
                    let name = parts[0].to_string();
                    let description = parts[4..].join(" ");
                    packages.push(Package {
                        name,
                        source: Source::Snap,
                        description,
                    });
                }
            }
        }
    }
    // Search Flatpak
    let flatpak_output = AsyncCommand::new("flatpak")
        .arg("search")
        .arg(&input)
        .output()
        .await
        .context("Failed to execute flatpak search")?;
    if flatpak_output.status.success() {
        let flatpak_str = String::from_utf8_lossy(&flatpak_output.stdout);
        let lines: Vec<&str> = flatpak_str.lines().collect();
        let start = if !lines.is_empty() && lines[0].contains("Name") { 1 } else { 0 };
        for line in &lines[start..] {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let parts: Vec<&str> = trimmed.split('\t').collect();
                if parts.len() >= 3 {
                    let name = parts[2].to_string(); // Application ID
                    let description = format!("{} - {}", parts.get(0).unwrap_or(&""), parts.get(1).unwrap_or(&""));
                    packages.push(Package {
                        name,
                        source: Source::Flatpak,
                        description,
                    });
                }
            }
        }
    }
    Ok(packages)
}

pub async fn install_package(pkg: Package) -> Result<String> {
    let (cmd, args) = match pkg.source {
        Source::Apt => ("apt", vec!["install", "-y", &pkg.name]),
        Source::Snap => ("snap", vec!["install", &pkg.name]),
        Source::Flatpak => ("flatpak", vec!["install", "--assumeyes", &pkg.name]),
        _ => return Ok("Invalid source".to_string()),
    };
    let output = AsyncCommand::new("sudo")
        .arg(cmd)
        .args(&args)
        .output()
        .await
        .context("Failed to install package")?;
    if output.status.success() {
        Ok(format!("Installed {} from {}", pkg.name, pkg.source.as_str()))
    } else {
        Err(anyhow::anyhow!("Failed to install {} from {}: {}", pkg.name, pkg.source.as_str(), String::from_utf8_lossy(&output.stderr)))
    }
}

pub async fn remove_package(pkg: Package) -> Result<String> {
    let (cmd, args) = match pkg.source {
        Source::Apt => ("apt", vec!["remove", "-y", &pkg.name]),
        Source::Snap => ("snap", vec!["remove", &pkg.name]),
        Source::Flatpak => ("flatpak", vec!["uninstall", "--assumeyes", &pkg.name]),
        _ => return Ok("Invalid source".to_string()),
    };
    let output = AsyncCommand::new("sudo")
        .arg(cmd)
        .args(&args)
        .output()
        .await
        .context("Failed to remove package")?;
    if output.status.success() {
        Ok(format!("Removed {} from {}", pkg.name, pkg.source.as_str()))
    } else {
        Err(anyhow::anyhow!("Failed to remove {} from {}: {}", pkg.name, pkg.source.as_str(), String::from_utf8_lossy(&output.stderr)))
    }
}
