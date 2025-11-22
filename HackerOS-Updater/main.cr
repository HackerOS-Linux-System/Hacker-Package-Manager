require "option_parser"
require "process"
require "file_utils"
require "colorize"

# ANSI color codes
RED = "\e[31m"
GREEN = "\e[32m"
BLUE = "\e[34m"
YELLOW = "\e[33m"
CYAN = "\e[36m"
MAGENTA = "\e[35m"
RESET = "\e[0m"

# Paths
HACKEROS_UPDATE_SCRIPT = "/usr/share/HackerOS/Scripts/Bin/update-hackeros.sh"
WALLPAPERS_UPDATE_SCRIPT = "/usr/share/HackerOS/Scripts/Bin/update-wallpapers.sh"
BIN_PATH = Process.executable_path.not_nil!
AUTO_SCRIPT_PATH = "#{ENV["HOME"]}/.hackeros/auto-update.sh" # Script to wait for internet

def display_header(title : String)
  puts "<--------[ #{title} ]-------->".colorize(:yellow)
end

def run_command(cmd : String) : {Bool, String}
  status = Process.run(cmd, shell: true, input: Process::Redirect::Inherit, output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  {status.success?, ""}
end

def get_status(success : Bool) : String
  if success
    "#{BLUE}COMPLETE#{RESET}"
  else
    "#{RED}FAILED#{RESET}"
  end
end

def perform_updates : {String, String, String, String, String, String, String}
  # APT Update
  display_header("System Update")
  apt_success = true
  ["sudo apt update", "sudo apt upgrade -y", "sudo apt autoclean"].each do |cmd|
    success, _ = run_command(cmd)
    apt_success &&= success
  end
  apt_status = get_status(apt_success)

  # Flatpak Update
  display_header("Flatpak Update")
  flatpak_success, _ = run_command("flatpak update -y")
  flatpak_status = get_status(flatpak_success)

  # Snap Update
  display_header("Snap Update")
  snap_success, _ = run_command("sudo snap refresh")
  snap_status = get_status(snap_success)

  # Firmware Update
  display_header("Firmware Update")
  fw_success, _ = run_command("sudo fwupdmgr update")
  fw_status = get_status(fw_success)

  # Oh My Zsh Update
  display_header("Oh My Zsh Update")
  omz_success, _ = run_command("omz update")
  omz_status = get_status(omz_success)

  # HackerOS Update
  display_header("HackerOS Update")
  hacker_success, _ = run_command(HACKEROS_UPDATE_SCRIPT)
  hacker_status = get_status(hacker_success)

  # Wallpapers Update
  display_header("Wallpaper Updates")
  wall_success, _ = run_command(WALLPAPERS_UPDATE_SCRIPT)
  wall_status = get_status(wall_success)

  {apt_status, flatpak_status, snap_status, fw_status, omz_status, hacker_status, wall_status}
end

def show_summary(apt_status, flatpak_status, snap_status, fw_status, omz_status, hacker_status, wall_status)
  puts "\nSystem Updates - #{apt_status}"
  puts "Flatpak Updates - #{flatpak_status}"
  puts "Snap Updates - #{snap_status}"
  puts "Firmware Updates - #{fw_status}"
  puts "Oh My Zsh Updates - #{omz_status}"
  puts "HackerOS Updates - #{hacker_status}"
  puts "Wallpaper Updates - #{wall_status}"
end

def enable_automatic_updates
  # Create a script that waits for internet and runs the updater
  auto_script = <<-SCRIPT
  #!/bin/bash
  while ! ping -c 1 google.com &> /dev/null; do
    sleep 5
  done
  #{BIN_PATH}
  SCRIPT
  File.write(AUTO_SCRIPT_PATH, auto_script)
  File.chmod(AUTO_SCRIPT_PATH, 0o755)

  # Add to crontab
  current_crontab = `crontab -l`
  unless current_crontab.includes?("@reboot #{AUTO_SCRIPT_PATH}")
    new_crontab = current_crontab + "\n@reboot #{AUTO_SCRIPT_PATH}\n"
    File.write("/tmp/crontab.txt", new_crontab)
    run_command("crontab /tmp/crontab.txt")
    File.delete("/tmp/crontab.txt")
  end
  puts "#{GREEN}Automatic updates enabled.#{RESET}"
end

def disable_automatic_updates
  # Remove from crontab
  current_crontab = `crontab -l`
  new_crontab = current_crontab.lines.reject { |line| line.includes?("@reboot #{AUTO_SCRIPT_PATH}") }.join("\n")
  File.write("/tmp/crontab.txt", new_crontab)
  run_command("crontab /tmp/crontab.txt")
  File.delete("/tmp/crontab.txt")

  # Remove script if exists
  File.delete(AUTO_SCRIPT_PATH) if File.exists?(AUTO_SCRIPT_PATH)
  puts "#{GREEN}Automatic updates disabled.#{RESET}"
end

def show_gui_menu
  loop do
    puts "\n#{YELLOW}=== HackerOS Updater Menu ===#{RESET}"
    puts "#{GREEN}[Q]uit#{RESET} #{CYAN}- Close this terminal#{RESET}"
    puts "#{GREEN}[R]eboot#{RESET} #{CYAN}- Reboot the system#{RESET}"
    puts "#{GREEN}[S]hutdown#{RESET} #{CYAN}- Shutdown the system#{RESET}"
    puts "#{GREEN}[L]og out#{RESET} #{CYAN}- Log out from current session#{RESET}"
    puts "#{GREEN}[T]erminal#{RESET} #{CYAN}- Open a new Alacritty terminal#{RESET}"
    puts "#{GREEN}[A]utomatic Updates#{RESET} #{CYAN}- Enable automatic updates on boot#{RESET}"
    print "#{MAGENTA}Enter your choice: #{RESET}"
    choice = ""
    STDIN.raw do |io|
      byte = io.read_byte
      if byte
        choice = byte.chr.to_s.upcase
        puts choice # Echo the choice
      end
    end
    case choice
    when "Q"
      exit(0)
    when "R"
      run_command("sudo reboot")
    when "S"
      run_command("sudo shutdown -h now")
    when "L"
      # For KDE
      run_command("qdbus org.kde.ksmserver /KSMServer logout 0 0 0")
    when "T"
      Process.new("alacritty", input: Process::Redirect::Close, output: Process::Redirect::Close, error: Process::Redirect::Close)
    when "A"
      enable_automatic_updates
    else
      puts "#{RED}Invalid choice. Try again.#{RESET}"
    end
  end
end

def main
  with_gui = false
  gui_mode = false
  disable_auto = false
  auto_mode = false
  OptionParser.parse do |parser|
    parser.banner = "Usage: HackerOS-Updater [options]"
    parser.on("--with-gui", "Run in GUI mode with Alacritty") { with_gui = true }
    parser.on("--gui-mode", "Internal GUI mode") { gui_mode = true }
    parser.on("--disable-automatic-update", "Disable automatic updates") { disable_auto = true }
    parser.on("--auto", "Run in automatic mode (internal)") { auto_mode = true }
  end

  if disable_auto
    disable_automatic_updates
    return
  end

  if with_gui
    # Launch in Alacritty with gui-mode
    Process.new("alacritty", args: ["-e", BIN_PATH, "--gui-mode"], input: Process::Redirect::Close, output: Process::Redirect::Close, error: Process::Redirect::Close)
    return
  end

  apt_status, flatpak_status, snap_status, fw_status, omz_status, hacker_status, wall_status = perform_updates
  show_summary(apt_status, flatpak_status, snap_status, fw_status, omz_status, hacker_status, wall_status)

  if gui_mode
    show_gui_menu
  end
end

main
