require "option_parser"

def display_help
  puts "\e[37m" + "-" * 50 + "\e[0m"
  puts "\e[1;35m hpm - Hackeros Package Manager \e[0m"
  puts "\e[37m" + "-" * 50 + "\e[0m"
  puts ""
  puts "\e[1;97mCommands:\e[0m"
  puts "  \e[35minstall {package}\e[0m     \e[37m- Installs the specified package\e[0m"
  puts "  \e[35mremove {package}\e[0m      \e[37m- Removes the specified package\e[0m"
  puts "  \e[35mupdate\e[0m                \e[37m- Updates the package list\e[0m"
  puts "  \e[35mclean\e[0m                 \e[37m- Cleans up the package cache\e[0m"
  puts "  \e[35mcommunity install {package}\e[0m \e[37m- Installs the specified community package\e[0m"
  puts "  \e[35mcommunity remove {package}\e[0m  \e[37m- Removes the specified community package\e[0m"
  puts "  \e[35mcommunity update\e[0m        \e[37m- Updates the community package list\e[0m"
  puts "  \e[35mcommunity clean\e[0m         \e[37m- Cleans up the community package cache\e[0m"
  puts "  \e[35mhelp\e[0m                  \e[37m- Shows this help message\e[0m"
  puts "  \e[35mtui\e[0m                   \e[37m- Launches the TUI interface\e[0m"
  puts ""
  puts "\e[1;97mUsage:\e[0m \e[37mhpm [arguments]\e[0m"
  puts "\e[37m" + "-" * 50 + "\e[0m"
end

def main
  if ARGV.empty?
    display_help
    exit(1)
  end

  command = ARGV.shift.downcase
  home_dir = File.expand_path("~")
  apt_frontend_path = File.join(home_dir, ".hackeros", "hpm", "apt-frontend")
  community_frontend_path = File.join(home_dir, ".hackeros", "hpm", "community-frontend")
  tui_path = File.join(home_dir, ".hackeros", "hpm", "tui")

  case command
  when "install"
    if ARGV.empty?
      puts "\e[1;90mError: Missing package name for install.\e[0m"
      exit(1)
    end
    package = ARGV.shift
    Process.run(apt_frontend_path, args: ["install", package], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  when "remove"
    if ARGV.empty?
      puts "\e[1;90mError: Missing package name for remove.\e[0m"
      exit(1)
    end
    package = ARGV.shift
    Process.run(apt_frontend_path, args: ["remove", package], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  when "update"
    Process.run(apt_frontend_path, args: ["update"], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  when "clean"
    Process.run(apt_frontend_path, args: ["clean"], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  when "community"
    if ARGV.empty?
      puts "\e[1;90mError: Missing subcommand for community.\e[0m"
      display_help
      exit(1)
    end
    subcommand = ARGV.shift.downcase
    case subcommand
    when "install"
      if ARGV.empty?
        puts "\e[1;90mError: Missing package name for community install.\e[0m"
        exit(1)
      end
      package = ARGV.shift
      Process.run(community_frontend_path, args: ["install", package], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
    when "remove"
      if ARGV.empty?
        puts "\e[1;90mError: Missing package name for community remove.\e[0m"
        exit(1)
      end
      package = ARGV.shift
      Process.run(community_frontend_path, args: ["remove", package], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
    when "update"
      Process.run(community_frontend_path, args: ["update"], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
    when "clean"
      Process.run(community_frontend_path, args: ["clean"], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
    else
      puts "\e[1;90mError: Unknown community subcommand '#{subcommand}'.\e[0m"
      display_help
      exit(1)
    end
  when "help"
    display_help
  when "tui"
    Process.run(tui_path, args: [] of String, output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  else
    puts "\e[1;90mError: Unknown command '#{command}'.\e[0m"
    display_help
    exit(1)
  end
end

main
