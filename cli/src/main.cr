require "option_parser"

def display_help
  puts "hpm - Hackeros Package Manager"
  puts ""
  puts "Commands:"
  puts "  install {package}  - Installs the specified package"
  puts "  remove {package}   - Removes the specified package"
  puts "  update             - Updates the package list"
  puts "  clean              - Cleans up the package cache"
  puts "  help               - Shows this help message"
  puts "  tui                - Launches the TUI interface"
  puts ""
  puts "Usage: hpm <command> [arguments]"
end

def main
  if ARGV.empty?
    display_help
    exit(1)
  end

  command = ARGV.shift.downcase
  home_dir = ENV.fetch("HOME", Dir.home)
  apt_frontend_path = File.join(home_dir, ".hackeros", "hpm", "apt-fronted")
  tui_path = File.join(home_dir, ".hackeros", "hpm", "tui")

  case command
  when "install"
    if ARGV.empty?
      puts "Error: Missing package name for install."
      exit(1)
    end
    package = ARGV.shift
    Process.run(apt_frontend_path, args: ["install", package], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)

  when "remove"
    if ARGV.empty?
      puts "Error: Missing package name for remove."
      exit(1)
    end
    package = ARGV.shift
    Process.run(apt_frontend_path, args: ["remove", package], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)

  when "update"
    Process.run(apt_frontend_path, args: ["update"], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)

  when "clean"
    Process.run(apt_frontend_path, args: ["clean"], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)

  when "help"
    display_help

  when "tui"
    Process.run(tui_path, args: [] of String, output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)

  else
    puts "Error: Unknown command '#{command}'."
    display_help
    exit(1)
  end
end

main
