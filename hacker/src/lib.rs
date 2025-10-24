mod commands;
mod game;
mod help;
mod utils;
pub use commands::{handle_unpack, handle_system, handle_run};
pub use game::play_game;
pub use help::display_ascii;
pub use utils::{handle_update, run_command_with_spinner};
use clap::Subcommand;
#[derive(Subcommand)]
pub enum UnpackCommands {
    /// Install add-ons (Wine, BoxBuddy, Winezgui, Gearlever)
    AddOns,
    /// Install both gaming and cybersecurity tools
    GS,
    /// Install development tools (Atom)
    Devtools,
    /// Install emulators (PlayStation, Nintendo, DOSBox, PS3)
    Emulators,
    /// Install cybersecurity tools (nmap, wireshark, Metasploit, Ghidra, etc.)
    Cybersecurity,
    /// Interactive UI for selecting individual packages
    Select,
    /// Install gaming tools (OBS Studio, Lutris, Steam, etc.) with Roblox
    Gaming,
    /// Install gaming tools without Roblox
    Noroblox,
    /// Install gamescope for hacker mode
    HackerMode,
}
#[derive(Subcommand)]
pub enum SystemCommands {
    /// Show system logs
    Logs,
}
#[derive(Subcommand)]
pub enum RunCommands {
    /// Run HackerOS Cockpit
    HackerosCockpit,
    /// Switch to another session
    SwitchToOtherSession,
    /// Update the system
    UpdateSystem,
    /// Check for system updates
    CheckUpdates,
    /// Launch Steam via HackerOS script
    Steam,
    /// Launch HackerOS Launcher
    HackerLauncher,
    /// Run HackerOS Game Mode
    HackerosGameMode,
}
