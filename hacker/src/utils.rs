use colored::*;
use std::process::{Command, Stdio};
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
pub fn run_command_with_spinner(program: &str, args: Vec<&str>, message: &str) {
    println!("{}", format!("▶ {}: {}", message, args.join(" ")).blue().bold().on_black());
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let spinner_chars: Vec<&str> = vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let handle = thread::spawn(move || {
        let mut i = 0;
        while !stop_clone.load(Ordering::Relaxed) {
            print!("\r{}", spinner_chars[i].purple().bold());
            let _ = io::stdout().flush();
            i = (i + 1) % spinner_chars.len();
            thread::sleep(Duration::from_millis(100));
        }
        print!("\r \r");
        let _ = io::stdout().flush();
    });
    let child = Command::new(program)
    .args(&args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect(&format!("Failed to execute {}", program));
    let output = child.wait_with_output().expect("Failed to wait on child");
    stop.store(true, Ordering::Relaxed);
    handle.join().unwrap();
    if output.status.success() {
        let out_str = String::from_utf8_lossy(&output.stdout).to_string();
        if !out_str.is_empty() {
            println!("{}", format!("┌── Output ────────────────").green().bold().on_black());
            println!("{}", out_str.green().on_black());
            println!("{}", format!("└──────────────────────────").green().bold().on_black());
        } else {
            println!("{}", "✔ Success (no output)".green().bold().on_black());
        }
    } else {
        let err_str = String::from_utf8_lossy(&output.stderr).to_string();
        println!("{}", format!("┌── Error ─────────────────").red().bold().on_black());
        println!("{}", err_str.red().on_black());
        println!("{}", format!("└──────────────────────────").red().bold().on_black());
    }
}
pub fn handle_update() {
    println!("{}", "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓".magenta().bold().on_black());
    println!("{}", "┃ Starting System Update ┃".magenta().bold().on_black());
    println!("{}", "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛".magenta().bold().on_black());
    run_command_with_spinner("sudo", vec!["apt", "update"], "Updating APT repositories");
    run_command_with_spinner("sudo", vec!["apt", "upgrade", "-y"], "Upgrading APT packages");
    run_command_with_spinner("flatpak", vec!["update", "-y"], "Updating Flatpak packages");
    run_command_with_spinner("snap", vec!["refresh"], "Refreshing Snap packages");
    run_command_with_spinner("fwupdmgr", vec!["update"], "Updating firmware");
    run_command_with_spinner("omz", vec!["update"], "Updating Oh-My-Zsh");
    run_command_with_spinner("sudo", vec!["/usr/share/HackerOS/Scripts/Bin/update-hackeros.sh"], "Updating HackerOS");
    println!("{}", "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓".green().bold().on_black());
    println!("{}", "┃ System Update Complete ┃".green().bold().on_black());
    println!("{}", "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛".green().bold().on_black());
}
pub fn handle_cybersecurity() {
    println!("{}", "========== Setting up Cybersecurity Container ==========".cyan().bold().on_black());
    run_command_with_spinner("sudo", vec!["apt", "install", "-y", "distrobox"], "Installing distrobox if not present");
    let exists = Command::new("distrobox").arg("list").output().map(|o| String::from_utf8_lossy(&o.stdout).contains("cybersecurity")).unwrap_or(false);
    if !exists {
        run_command_with_spinner("distrobox", vec!["create", "--name", "cybersecurity", "--image", "archlinux:latest"], "Creating container");
    }
    let ba_check = Command::new("distrobox-enter").args(&["-n", "cybersecurity", "--", "sudo", "grep", "\\[blackarch\\]", "/etc/pacman.conf"]).output().map(|o| o.status.success()).unwrap_or(false);
    if !ba_check {
        run_command_with_spinner("distrobox-enter", vec!["-n", "cybersecurity", "--", "curl", "-O", "https://blackarch.org/strap.sh"], "Downloading strap.sh");
        run_command_with_spinner("distrobox-enter", vec!["-n", "cybersecurity", "--", "bash", "-c", "echo e26445d34490cc06bd14b51f9924debf569e0ecb strap.sh | sha1sum -c"], "Verifying sha1sum");
        run_command_with_spinner("distrobox-enter", vec!["-n", "cybersecurity", "--", "chmod", "+x", "strap.sh"], "Making executable");
        run_command_with_spinner("distrobox-enter", vec!["-n", "cybersecurity", "--", "sudo", "./strap.sh"], "Installing BlackArch");
    }
    let multi_check = Command::new("distrobox-enter").args(&["-n", "cybersecurity", "--", "grep", "^\\[multilib\\]", "/etc/pacman.conf"]).output().map(|o| o.status.success()).unwrap_or(false);
    if !multi_check {
        run_command_with_spinner("distrobox-enter", vec!["-n", "cybersecurity", "--", "sudo", "sed", "-i", "/#\\[multilib\\]/,/Include/s/^#//", "/etc/pacman.conf"], "Enabling multilib");
    }
    run_command_with_spinner("distrobox-enter", vec!["-n", "cybersecurity", "--", "sudo", "pacman", "-Syu", "--noconfirm"], "Updating system");
    let home = std::env::var("HOME").unwrap_or_default();
    let select_bin = format!("{}/.hackeros/hacker/hacker-select", home);
    let select_output = Command::new(&select_bin).arg("-mode").arg("cyber").output().expect("Failed to run hacker-select");
    if !select_output.status.success() {
        println!("{}", "Error running hacker-select".red().bold().on_black());
        return;
    }
    let selected_str = String::from_utf8_lossy(&select_output.stdout).to_string();
    let selected: Vec<String> = selected_str.lines().map(|s| s.to_string()).collect();
    if selected.is_empty() {
        println!("{}", "No tools selected.".yellow().bold().on_black());
        return;
    }
    if selected.contains(&"all".to_string()) {
        run_command_with_spinner("distrobox-enter", vec!["-n", "cybersecurity", "--", "sudo", "pacman", "-S", "--noconfirm", "blackarch"], "Installing all BlackArch tools");
    } else {
        let mut args: Vec<&str> = vec!["-n", "cybersecurity", "--", "sudo", "pacman", "-S", "--noconfirm", "--needed"];
        for s in &selected {
            args.push(s);
        }
        run_command_with_spinner("distrobox-enter", args.into_iter().map(|s| s).collect(), "Installing selected categories");
    }
    println!("{}", "========== Cybersecurity Setup Complete ==========".green().bold().on_black());
}
pub fn handle_gaming() {
    println!("{}", "========== Installing Gaming Tools ==========".cyan().bold().on_black());
    run_command_with_spinner("flatpak", vec!["remote-add", "--if-not-exists", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo"], "Adding flathub repo");
    run_command_with_spinner("sudo", vec!["apt", "install", "-y", "obs-studio", "lutris"], "Installing OBS Studio and Lutris");
    run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "com.valvesoftware.Steam"], "Installing Steam");
    run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "io.github.giantpinkrobots.varia"], "Installing Pika Torrent");
    run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "net.davidotek.pupgui2"], "Installing ProtonUp-Qt");
    run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "com.heroicgameslauncher.hgl", "protontricks", "com.discordapp.Discord"], "Installing Heroic Games Launcher, Protontricks, and Discord");
    run_command_with_spinner("flatpak", vec!["install", "--user", "https://sober.vinegarhq.org/sober.flatpakref"], "Installing Roblox");
    run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "org.vinegarhq.Vinegar"], "Installing Roblox Studio");
    println!("{}", "========== Hacker-Unpack-Gaming Complete ==========".green().bold().on_black());
}
