use colored::*;
use crate::utils::{handle_cybersecurity, handle_gaming, run_command_with_spinner};
use crate::UnpackCommands;
use crate::SystemCommands;
use crate::RunCommands;
use crate::PluginCommands;
use std::process::Command;
use std::path::Path;
use std::os::unix::fs::symlink;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Plugin {
    description: Option<String>,
    commands: Vec<PluginCommand>,
}

#[derive(Serialize, Deserialize)]
struct PluginCommand {
    program: String,
    args: Vec<String>,
    message: String,
}

pub fn handle_unpack(unpack_command: UnpackCommands) {
    match unpack_command {
        UnpackCommands::AddOns => {
            println!("{}", "========== Installing Add-Ons ==========".cyan().bold().on_black());
            run_command_with_spinner("flatpak", vec!["remote-add", "--if-not-exists", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo"], "Adding flathub repo");
            run_command_with_spinner("sudo", vec!["apt", "install", "-y", "wine", "winetricks"], "Installing Wine");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "io.github.dvlv.boxbuddyrs"], "Installing BoxBuddy");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "it.mijorus.winezgui"], "Installing Winezgui");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "it.mijorus.gearlever"], "Installing Gearlever");
            println!("{}", "========== Install Add-Ons Complete ==========".green().bold().on_black());
        }
        UnpackCommands::GS => {
            handle_gaming();
            handle_cybersecurity();
        }
        UnpackCommands::Devtools => {
            println!("{}", "========== Installing Atom ==========".cyan().bold().on_black());
            run_command_with_spinner("flatpak", vec!["remote-add", "--if-not-exists", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo"], "Adding flathub repo");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "io.atom.Atom"], "Installing Atom");
            println!("{}", "========== Install Dev Tools Complete ==========".green().bold().on_black());
        }
        UnpackCommands::Emulators => {
            println!("{}", "========== Installing Emulators ==========".cyan().bold().on_black());
            run_command_with_spinner("flatpak", vec!["remote-add", "--if-not-exists", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo"], "Adding flathub repo");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "org.shadps4.shadPS4"], "Installing PlayStation Emulator");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "io.github.ryubing.Ryujinx"], "Installing Nintendo Emulator");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "com.dosbox_x.DOSBox-X"], "Installing DOSBox");
            run_command_with_spinner("sudo", vec!["snap", "install", "rpcs3-emu"], "Installing PlayStation 3 Emulator");
            println!("{}", "========== Hacker-Unpack-Emulators Complete ==========".green().bold().on_black());
        }
        UnpackCommands::Cybersecurity => {
            handle_cybersecurity();
        }
        UnpackCommands::Select => {
            println!("{}", "========== Interactive Package Selection ==========".yellow().bold().on_black());
            let home = std::env::var("HOME").unwrap_or_default();
            let select_bin = format!("{}/.hackeros/hacker/hacker-select", home);
            let select_output = Command::new(&select_bin).arg("-mode").arg("unpack").output().expect("Failed to run hacker-select");
            if !select_output.status.success() {
                println!("{}", "Error running hacker-select".red().bold().on_black());
                return;
            }
            let selected_str = String::from_utf8_lossy(&select_output.stdout).to_string();
            let selected: Vec<String> = selected_str.lines().map(|s| s.to_string()).collect();
            for s in selected {
                if s.starts_with("category:") {
                    let group = s.strip_prefix("category:").unwrap().trim().to_lowercase();
                    match group.as_str() {
                        "add-ons" => handle_unpack(UnpackCommands::AddOns),
                        "gaming" => handle_unpack(UnpackCommands::Gaming),
                        "cybersecurity" => handle_unpack(UnpackCommands::Cybersecurity),
                        "devtools" => handle_unpack(UnpackCommands::Devtools),
                        "emulators" => handle_unpack(UnpackCommands::Emulators),
                        "hacker-mode" => handle_unpack(UnpackCommands::HackerMode),
                        "noroblox" => handle_unpack(UnpackCommands::Noroblox),
                        _ => println!("{}", format!("Unknown category: {}", group).red().bold()),
                    }
                } else if s.starts_with("app:") {
                    let app = s.strip_prefix("app:").unwrap().trim();
                    match app {
                        "wine" => run_command_with_spinner("sudo", vec!["apt", "install", "-y", "wine"], "Installing wine"),
                        "winetricks" => run_command_with_spinner("sudo", vec!["apt", "install", "-y", "winetricks"], "Installing winetricks"),
                        "io.github.dvlv.boxbuddyrs" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "io.github.dvlv.boxbuddyrs"], "Installing BoxBuddy"),
                        "it.mijorus.winezgui" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "it.mijorus.winezgui"], "Installing Winezgui"),
                        "it.mijorus.gearlever" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "it.mijorus.gearlever"], "Installing Gearlever"),
                        "obs-studio" => run_command_with_spinner("sudo", vec!["apt", "install", "-y", "obs-studio"], "Installing obs-studio"),
                        "lutris" => run_command_with_spinner("sudo", vec!["apt", "install", "-y", "lutris"], "Installing lutris"),
                        "com.valvesoftware.Steam" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "com.valvesoftware.Steam"], "Installing Steam"),
                        "io.github.giantpinkrobots.varia" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "io.github.giantpinkrobots.varia"], "Installing Pika Torrent"),
                        "net.davidotek.pupgui2" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "net.davidotek.pupgui2"], "Installing ProtonUp-Qt"),
                        "com.heroicgameslauncher.hgl" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "com.heroicgameslauncher.hgl"], "Installing Heroic Games Launcher"),
                        "protontricks" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "protontricks"], "Installing protontricks"),
                        "com.discordapp.Discord" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "com.discordapp.Discord"], "Installing Discord"),
                        "roblox" => run_command_with_spinner("flatpak", vec!["install", "--user", "https://sober.vinegarhq.org/sober.flatpakref"], "Installing Roblox"),
                        "roblox-studio" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "org.vinegarhq.Vinegar"], "Installing Roblox Studio"),
                        "io.atom.Atom" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "io.atom.Atom"], "Installing Atom"),
                        "org.shadps4.shadPS4" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "org.shadps4.shadPS4"], "Installing shadPS4"),
                        "io.github.ryubing.Ryujinx" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "io.github.ryubing.Ryujinx"], "Installing Ryujinx"),
                        "com.dosbox_x.DOSBox-X" => run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "com.dosbox_x.DOSBox-X"], "Installing DOSBox-X"),
                        "rpcs3-emu" => run_command_with_spinner("sudo", vec!["snap", "install", "rpcs3-emu"], "Installing RPCS3"),
                        "gamescope" => run_command_with_spinner("sudo", vec!["apt", "install", "-y", "gamescope"], "Installing gamescope"),
                        _ => println!("{}", format!("Unknown app: {}", app).red().bold()),
                    }
                }
            }
            println!("{}", "========== Selection Complete ==========".green().bold().on_black());
        }
        UnpackCommands::Gaming => {
            handle_gaming();
        }
        UnpackCommands::Noroblox => {
            println!("{}", "========== Installing Gaming Tools (No Roblox) ==========".cyan().bold().on_black());
            run_command_with_spinner("flatpak", vec!["remote-add", "--if-not-exists", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo"], "Adding flathub repo");
            run_command_with_spinner("sudo", vec!["apt", "install", "-y", "obs-studio", "lutris"], "Installing OBS Studio and Lutris");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "com.valvesoftware.Steam"], "Installing Steam");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "io.github.giantpinkrobots.varia"], "Installing Pika Torrent");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "net.davidotek.pupgui2"], "Installing ProtonUp-Qt");
            run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "com.heroicgameslauncher.hgl", "protontricks", "com.discordapp.Discord"], "Installing Heroic Games Launcher, Protontricks, and Discord");
            println!("{}", "========== Hacker-Unpack-Gaming-NoRoblox Complete ==========".green().bold().on_black());
        }
        UnpackCommands::HackerMode => {
            println!("{}", "========== Installing Hacker Mode ==========".cyan().bold().on_black());
            run_command_with_spinner("sudo", vec!["apt", "install", "-y", "gamescope"], "Installing gamescope");
            println!("{}", "========== Hacker Mode Install Complete ==========".green().bold().on_black());
        }
        UnpackCommands::GamescopeSessionSteam => {
            println!("{}", "========== Setting up gamescope-session-steam ==========".cyan().bold().on_black());
            // Check if gamescope is installed
            let gamescope_installed = Command::new("gamescope").arg("--version").status().map(|s| s.success()).unwrap_or(false);
            if !gamescope_installed {
                run_command_with_spinner("sudo", vec!["apt", "install", "-y", "gamescope"], "Installing gamescope");
            } else {
                println!("{}", "gamescope is already installed.".green().bold().on_black());
            }
            // Check if Steam flatpak is installed
            let steam_output = Command::new("flatpak").arg("list").output().map(|o| String::from_utf8_lossy(&o.stdout).contains("com.valvesoftware.Steam")).unwrap_or(false);
            if !steam_output {
                run_command_with_spinner("flatpak", vec!["install", "-y", "flathub", "com.valvesoftware.Steam"], "Installing Steam flatpak");
            } else {
                println!("{}", "Steam flatpak is already installed.".green().bold().on_black());
            }
            // Clone repo to /tmp/
            let repo_dir = "/tmp/gamescope-session-steam";
            // Remove if exists
            let _ = Command::new("rm").args(&["-rf", repo_dir]).output();
            run_command_with_spinner("git", vec!["clone", "https://github.com/HackerOS-Linux-System/gamescope-session-steam.git", repo_dir], "Cloning repository");
            // Run hackerc run unpack.hacker
            let unpack_path = format!("{}/unpack.hacker", repo_dir);
            run_command_with_spinner("hackerc", vec!["run", &unpack_path], "Running unpack.hacker");
            println!("{}", "========== gamescope-session-steam Setup Complete ==========".green().bold().on_black());
        }
        UnpackCommands::Xanmod => {
            println!("{}", "========== Unpacking Xanmod ==========".cyan().bold().on_black());
            run_command_with_spinner("sudo", vec!["/usr/share/HackerOS/Scripts/Bin/unpack-xanmod.sh"], "Running unpack-xanmod.sh");
            println!("{}", "========== Xanmod Unpack Complete ==========".green().bold().on_black());
        }
        UnpackCommands::Liquorix => {
            println!("{}", "========== Unpacking Liquorix ==========".cyan().bold().on_black());
            run_command_with_spinner("sudo", vec!["/usr/share/HackerOS/Scripts/Bin/unpack-liquorix.sh"], "Running unpack-liquorix.sh");
            println!("{}", "========== Liquorix Unpack Complete ==========".green().bold().on_black());
        }
    }
}

pub fn handle_system(system_command: SystemCommands) {
    match system_command {
        SystemCommands::Logs => {
            println!("{}", "========== System Logs ==========".cyan().bold().on_black());
            run_command_with_spinner("sudo", vec!["journalctl", "-xe"], "Displaying system logs");
        }
    }
}

pub fn handle_run(cmd: RunCommands) {
    match cmd {
        RunCommands::UpdateSystem => run_command_with_spinner("sudo", vec!["/usr/share/HackerOS/Scripts/Bin/update-system.sh"], "Updating system"),
        RunCommands::CheckUpdates => run_command_with_spinner("sudo", vec!["/usr/share/HackerOS/Scripts/Bin/check_updates_notify.sh"], "Checking for updates"),
        RunCommands::Steam => run_command_with_spinner("bash", vec!["/usr/share/HackerOS/Scripts/Steam/HackerOS-Steam.sh"], "Launching Steam"),
        RunCommands::HackerLauncher => run_command_with_spinner("bash", vec!["/usr/share/HackerOS/Scripts/HackerOS-Apps/Hacker_Launcher"], "Launching HackerOS Launcher"),
        RunCommands::HackerosGameMode => run_command_with_spinner("", vec!["/usr/share/HackerOS/Scripts/HackerOS-Apps/HackerOS-Game-Mode.AppImage"], "Running HackerOS Game Mode"),
        RunCommands::UpdateHackeros => run_command_with_spinner("sudo", vec!["/usr/share/HackerOS/Scripts/Bin/update-hackeros.sh"], "Updating HackerOS"),
        RunCommands::UpdateWallpapers => run_command_with_spinner("sudo", vec!["/usr/share/HackerOS/Scripts/Bin/update-wallpapers.sh"], "Updating wallpapers"),
    }
}

pub fn handle_plugin(plugin_command: PluginCommands) {
    let home = std::env::var("HOME").unwrap_or_default();
    let config_dir = format!("{}/.config/hacker", home);
    std::fs::create_dir_all(&config_dir).expect("Failed to create config dir");
    match plugin_command {
        PluginCommands::Create { name } => {
            let path = format!("{}/{}.yaml", config_dir, name);
            if Path::new(&path).exists() {
                println!("{}", format!("Plugin {} already exists.", name).red().bold().on_black());
                return;
            }
            let template = Plugin {
                description: Some("Example plugin".to_string()),
                commands: vec![PluginCommand {
                    program: "sudo".to_string(),
                    args: vec!["apt".to_string(), "install".to_string(), "-y".to_string(), "vim".to_string()],
                    message: "Installing vim".to_string(),
                }],
            };
            let yaml = serde_yaml::to_string(&template).expect("Failed to serialize template");
            std::fs::write(&path, yaml).expect("Failed to write template");
            println!("{}", format!("Created plugin template at {}", path).green().bold().on_black());
        }
        PluginCommands::Enable { name } => {
            let plugin_file = format!("{}/{}.yaml", config_dir, name);
            if !Path::new(&plugin_file).exists() {
                println!("{}", format!("Plugin {} does not exist.", name).red().bold().on_black());
                return;
            }
            let enabled_dir = format!("{}/enabled", config_dir);
            std::fs::create_dir_all(&enabled_dir).expect("Failed to create enabled dir");
            let enabled_file = format!("{}/{}.yaml", enabled_dir, name);
            if Path::new(&enabled_file).exists() {
                println!("{}", format!("Plugin {} is already enabled.", name).yellow().bold().on_black());
                return;
            }
            symlink(&plugin_file, &enabled_file).expect("Failed to create symlink");
            println!("{}", format!("Enabled plugin {}", name).green().bold().on_black());
        }
        PluginCommands::Disable { name } => {
            let enabled_dir = format!("{}/enabled", config_dir);
            let enabled_file = format!("{}/{}.yaml", enabled_dir, name);
            if !Path::new(&enabled_file).exists() {
                println!("{}", format!("Plugin {} is not enabled.", name).yellow().bold().on_black());
                return;
            }
            std::fs::remove_file(&enabled_file).expect("Failed to remove symlink");
            println!("{}", format!("Disabled plugin {}", name).green().bold().on_black());
        }
        PluginCommands::List => {
            println!("{}", "Available plugins:".cyan().bold().on_black());
            let entries = std::fs::read_dir(&config_dir).expect("Failed to read config dir");
            for entry in entries {
                let path = entry.expect("Failed to get entry").path();
                if path.extension().and_then(|s| s.to_str()) == Some("yaml") && !path.is_dir() {
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                    if name != "config" { // Skip if any config.yaml
                        println!("{}", format!("- {}", name).white().bold());
                    }
                }
            }
            let enabled_dir = format!("{}/enabled", config_dir);
            if Path::new(&enabled_dir).exists() {
                println!("{}", "Enabled plugins:".cyan().bold().on_black());
                let enabled_entries = std::fs::read_dir(&enabled_dir).expect("Failed to read enabled dir");
                for entry in enabled_entries {
                    let path = entry.expect("Failed to get entry").path();
                    if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                        let name = path.file_stem().unwrap().to_string_lossy().to_string();
                        println!("{}", format!("- {}", name).white().bold());
                    }
                }
            } else {
                println!("{}", "No enabled plugins.".yellow().bold().on_black());
            }
        }
        PluginCommands::Apply => {
            let enabled_dir = format!("{}/enabled", config_dir);
            if !Path::new(&enabled_dir).exists() {
                println!("{}", "No enabled plugins.".yellow().bold().on_black());
                return;
            }
            let entries = std::fs::read_dir(&enabled_dir).expect("Failed to read enabled dir");
            for entry in entries {
                let path = entry.expect("Failed to get entry").path();
                if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                    let content = std::fs::read_to_string(&path).expect("Failed to read plugin file");
                    let plugin: Plugin = serde_yaml::from_str(&content).expect("Failed to parse YAML");
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                    println!("{}", format!("Applying plugin {}: {}", name, plugin.description.unwrap_or_default()).cyan().bold().on_black());
                    for cmd in plugin.commands {
                        run_command_with_spinner(&cmd.program, cmd.args.iter().map(|s| s.as_str()).collect(), &cmd.message);
                    }
                }
            }
            println!("{}", "All enabled plugins applied.".green().bold().on_black());
        }
    }
}
