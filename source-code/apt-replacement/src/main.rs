use clap::{Parser, Subcommand};
use colored::*;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use anyhow::{Context, Result, anyhow};
use chrono;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use reqwest::blocking::Client;

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

fn run_command(cmd: &str, args: &[&str], log_file: &mut File, description: &str, use_sudo: bool) -> Result<String> {
    let full_cmd = if use_sudo { format!("sudo {}", cmd) } else { cmd.to_string() };
    println!("{}", description.yellow().bold());

    let mut command = if use_sudo {
        let mut c = Command::new("sudo");
        c.arg(cmd);
        c
    } else {
        Command::new(cmd)
    };

    command.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command.output().context(format!("Failed to execute {}", full_cmd))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    writeln!(log_file, "Command: {} {}\nStdout: {}\nStderr: {}", full_cmd, args.join(" "), stdout, stderr)?;

    if !output.status.success() {
        println!("{}", stderr.red());
        return Err(anyhow!("Command failed: {}", stderr));
    }

    println!("{}", stdout.green());
    Ok(stdout)
}

fn get_command_output(cmd: &str, args: &[&str], log_file: &mut File, use_sudo: bool) -> Result<String> {
    let full_cmd = if use_sudo { format!("sudo {}", cmd) } else { cmd.to_string() };
    let mut command = if use_sudo {
        let mut c = Command::new("sudo");
        c.arg(cmd);
        c
    } else {
        Command::new(cmd)
    };

    command.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command.output().context(format!("Failed to execute {}", full_cmd))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    writeln!(log_file, "Command: {} {}\nStdout: {}\nStderr: {}", full_cmd, args.join(" "), stdout, stderr)?;

    if !output.status.success() {
        return Err(anyhow!("Command failed: {}", stderr));
    }

    Ok(stdout)
}

fn is_package_installed(package: &str, log_file: &mut File) -> Result<bool> {
    // Use dpkg-query for read-only check
    let output = get_command_output("dpkg-query", &["-W", "-f=${Status}", package], log_file, false)?;

    let status = output.trim().to_string();
    writeln!(log_file, "Package status for {}: {}", package, status)?;

    Ok(status.contains("install ok installed"))
}

struct DownloadItem {
    url: String,
    filename: String,
    size: u64,
}

fn parse_print_uris_output(output: &str) -> Vec<DownloadItem> {
    let mut downloads = Vec::new();
    for line in output.lines() {
        if line.starts_with("'") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let url = parts[0].trim_matches('\'').to_string();
                let filename = parts[1].to_string();
                let size: u64 = parts[2].parse().unwrap_or(0);
                downloads.push(DownloadItem { url, filename, size });
            }
        }
    }
    downloads
}

fn download_with_progress(downloads: &[DownloadItem], log_file: &mut File) -> Result<Vec<String>> {
    let m = MultiProgress::new();
    let client = Client::new();
    let mut paths = Vec::new();

    // Sequential downloads for simplicity
    for item in downloads {
        let pb = m.add(ProgressBar::new(item.size));
        pb.set_style(
            ProgressStyle::with_template(
                "{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}) {eta}"
            )
            .unwrap()
            .progress_chars("##-")
        );
        pb.set_message(format!("Downloading {}", item.filename.green()));

        let path = format!("{}/{}", CACHE_DIR, item.filename);
        let mut file = BufWriter::new(File::create(&path).context("Failed to create file")?);

        let mut response = client.get(&item.url).send().context("Failed to send request")?.error_for_status().context("Bad response status")?;

        let mut downloaded: u64 = 0;
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = response.read(&mut buffer).context("Failed to read response")?;
            if bytes_read == 0 {
                break;
            }
            file.write_all(&buffer[0..bytes_read]).context("Failed to write to file")?;
            downloaded += bytes_read as u64;
            pb.set_position(downloaded.min(item.size));
        }

        file.flush().context("Failed to flush file")?;
        pb.finish_with_message(format!("Downloaded {}", item.filename.green()));
        paths.push(path);
    }

    writeln!(log_file, "Downloaded files: {:?}", paths)?;
    Ok(paths)
}

fn install_package(package: &str, log_file: &mut File) -> Result<()> {
    if is_package_installed(package, log_file)? {
        println!("{} {} {}", "Package".green(), package.cyan().bold(), "is already installed.".green());
        return Ok(());
    }

    // Refresh first
    refresh_packages(log_file)?;

    // Get URIs to download
    let uris_output = get_command_output("apt-get", &["--print-uris", "-y", "install", package], log_file, false)?;
    let downloads = parse_print_uris_output(&uris_output);

    if downloads.is_empty() {
        println!("{} {}", "No packages to download for".yellow(), package.cyan().bold());
        return Ok(());
    }

    // Download with progress
    let deb_paths = download_with_progress(&downloads, log_file)?;

    // Install using dpkg
    let mut args = vec!["-i"];
    for path in &deb_paths {
        args.push(path.as_str());
    }
    run_command("dpkg", &args, log_file, &format!("Installing {}", package), true)?;

    // Clean up deb files
    for path in deb_paths {
        fs::remove_file(&path).context("Failed to remove deb file")?;
    }

    println!("{} {} {}", "Successfully installed".green().bold(), package.cyan().bold(), "!".green().bold());
    Ok(())
}

fn remove_package(package: &str, log_file: &mut File) -> Result<()> {
    if !is_package_installed(package, log_file)? {
        println!("{} {} {}", "Package".red(), package.cyan().bold(), "is not installed.".red());
        return Ok(());
    }

    run_command("dpkg", &["--remove", package], log_file, &format!("Removing {}", package), true)?;

    println!("{} {} {}", "Successfully removed".red().bold(), package.cyan().bold(), "!".red().bold());
    Ok(())
}

fn clean_packages(log_file: &mut File) -> Result<()> {
    run_command("apt", &["autoclean"], log_file, "Running autoclean", true)?;
    run_command("apt", &["autoremove"], log_file, "Running autoremove", true)?;
    println!("{}", "Cleaned up packages!".blue().bold());
    Ok(())
}

fn update_packages(log_file: &mut File) -> Result<()> {
    refresh_packages(log_file)?;

    // Check if updates available
    let sim_output = get_command_output("apt-get", &["-s", "-y", "upgrade"], log_file, false)?;
    if !sim_output.contains("Inst ") {
        println!("{}", "All packages are up to date.".green().bold());
        return Ok(());
    }

    // Get URIs for upgrade
    let uris_output = get_command_output("apt-get", &["--print-uris", "-y", "upgrade"], log_file, false)?;
    let downloads = parse_print_uris_output(&uris_output);

    if downloads.is_empty() {
        println!("{}", "No updates to download.".yellow());
        return Ok(());
    }

    // Download with progress
    let deb_paths = download_with_progress(&downloads, log_file)?;

    // Install using dpkg (though usually apt upgrade handles, but to follow dpkg)
    // But dpkg may not handle upgrades properly if conflicts, but for simplicity
    // Actually, for upgrades, better to use apt upgrade, but to follow instructions, use dpkg.
    // But dpkg -i on upgrades works if it's upgrade.
    let mut args = vec!["-i"];
    for path in &deb_paths {
        args.push(path.as_str());
    }
    run_command("dpkg", &args, log_file, "Upgrading packages", true)?;

    // Clean up
    for path in deb_paths {
        fs::remove_file(&path).context("Failed to remove deb file")?;
    }

    println!("{}", "Packages updated!".magenta().bold());
    Ok(())
}

fn refresh_packages(log_file: &mut File) -> Result<()> {
    run_command("apt", &["update"], log_file, "Refreshing package lists", true)?;
    println!("{}", "Package lists refreshed!".cyan().bold());
    Ok(())
}
