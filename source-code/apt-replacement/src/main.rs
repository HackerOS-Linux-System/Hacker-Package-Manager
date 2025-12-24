use clap::{Parser, Subcommand};
use colored::*;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use anyhow::{Context, Result};

const CACHE_DIR: &str = "/var/cache/hpm/";
const LOG_DIR: &str = "/tmp/hpm/logs/";
const BIN_DIR: &str = "~/.hackeros/hpm/"; // Note: This is for installation reference, not used in code execution

#[derive(Parser)]
#[command(name = "hpm")]
#[command(about = "Hacker Package Manager - A colorful alternative to apt", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a package
    Install {
        /// Package name to install
        package: String,
    },
    /// Remove a package
    Remove {
        /// Package name to remove
        package: String,
    },
    /// Clean up unnecessary packages (like apt autoclean and autoremove)
    Clean,
    /// Update packages (refresh and upgrade)
    Update,
    /// Refresh package lists
    Refresh,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Ensure directories exist
    fs::create_dir_all(CACHE_DIR).context("Failed to create cache directory")?;
    fs::create_dir_all(LOG_DIR).context("Failed to create log directory")?;

    // Log file setup
    let log_path = format!("{}/hpm_{}.log", LOG_DIR, chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let mut log_file = File::create(&log_path).context("Failed to create log file")?;

    writeln!(log_file, "Starting hpm command: {:?}", std::env::args().collect::<Vec<_>>())?;

    match cli.command {
        Commands::Install { package } => install_package(&package, &mut log_file)?,
        Commands::Remove { package } => remove_package(&package, &mut log_file)?,
        Commands::Clean => clean_packages(&mut log_file)?,
        Commands::Update => update_packages(&mut log_file)?,
        Commands::Refresh => refresh_packages(&mut log_file)?,
    }

    writeln!(log_file, "Command completed successfully")?;
    Ok(())
}

fn run_command(cmd: &str, args: &[&str], log_file: &mut File, description: &str) -> Result<String> {
    println!("{}", description.yellow().bold());

    let output = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context(format!("Failed to execute {}", cmd))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    writeln!(log_file, "Command: {} {}\nStdout: {}\nStderr: {}", cmd, args.join(" "), stdout, stderr)?;

    if !output.status.success() {
        println!("{}", stderr.red());
        return Err(anyhow::anyhow!("Command failed: {}", stderr));
    }

    println!("{}", stdout.green());
    Ok(stdout)
}

fn is_package_installed(package: &str, log_file: &mut File) -> Result<bool> {
    // Use dpkg-query for read-only check (libapt equivalent in command form)
    let output = Command::new("dpkg-query")
        .args(&["-W", "-f=${Status}", package])
        .output()
        .context("Failed to check if package is installed")?;

    let status = String::from_utf8_lossy(&output.stdout).to_string();
    writeln!(log_file, "Package status for {}: {}", package, status)?;

    Ok(status.contains("install ok installed"))
}

fn install_package(package: &str, log_file: &mut File) -> Result<()> {
    if is_package_installed(package, log_file)? {
        println!("{} {}", "Package".green(), package.cyan().bold(), "is already installed.".green());
        return Ok(());
    }

    // Refresh first
    refresh_packages(log_file)?;

    // Download the package to cache (using apt download as read-only-ish)
    let deb_file = format!("{}/{}_latest.deb", CACHE_DIR, package);
    run_command("apt", &["download", package], log_file, &format!("Downloading {}", package))?;
    // Assume the deb is downloaded to current dir, move to cache
    if Path::new(&format!("{}_*.deb", package)).exists() {
        fs::rename(format!("{}_*.deb", package), &deb_file)?;
    } else {
        return Err(anyhow::anyhow!("Failed to download package"));
    }

    // Install using dpkg
    run_command("sudo", &["dpkg", "-i", &deb_file], log_file, &format!("Installing {}", package))?;

    // Clean up deb file? Optional
    fs::remove_file(&deb_file)?;

    println!("{} {} {}", "Successfully installed".green().bold(), package.cyan().bold(), "!".green().bold());
    Ok(())
}

fn remove_package(package: &str, log_file: &mut File) -> Result<()> {
    if !is_package_installed(package, log_file)? {
        println!("{} {}", "Package".red(), package.cyan().bold(), "is not installed.".red());
        return Ok(());
    }

    run_command("sudo", &["dpkg", "--remove", package], log_file, &format!("Removing {}", package))?;

    println!("{} {} {}", "Successfully removed".red().bold(), package.cyan().bold(), "!".red().bold());
    Ok(())
}

fn clean_packages(log_file: &mut File) -> Result<()> {
    run_command("sudo", &["apt", "autoclean"], log_file, "Running autoclean")?;
    run_command("sudo", &["apt", "autoremove"], log_file, "Running autoremove")?;
    println!("{}", "Cleaned up packages!".blue().bold());
    Ok(())
}

fn update_packages(log_file: &mut File) -> Result<()> {
    refresh_packages(log_file)?;
    run_command("sudo", &["apt", "upgrade", "-y"], log_file, "Upgrading packages")?;
    println!("{}", "Packages updated!".magenta().bold());
    Ok(())
}

fn refresh_packages(log_file: &mut File) -> Result<()> {
    run_command("sudo", &["apt", "update"], log_file, "Refreshing package lists")?;
    println!("{}", "Package lists refreshed!".cyan().bold());
    Ok(())
  }
