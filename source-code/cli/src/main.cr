require "process"

module Colors
  RED = "\e[31m"
  GREEN = "\e[32m"
  YELLOW = "\e[33m"
  BLUE = "\e[34m"
  MAGENTA = "\e[35m"
  CYAN = "\e[36m"
  RESET = "\e[0m"
end

def print_help
  puts "#{Colors::GREEN}Usage: hpm <command> [args]#{Colors::RESET}"
  puts "#{Colors::CYAN}Commands:#{Colors::RESET}"
  puts "  #{Colors::YELLOW}install#{Colors::RESET}     - Install packages"
  puts "  #{Colors::YELLOW}remove#{Colors::RESET}      - Remove packages"
  puts "  #{Colors::YELLOW}clean#{Colors::RESET}       - Clean up"
  puts "  #{Colors::YELLOW}update#{Colors::RESET}      - Update packages"
  puts "  #{Colors::YELLOW}tui#{Colors::RESET}         - Launch TUI interface"
  puts "  #{Colors::YELLOW}community#{Colors::RESET}   - Community commands: update, install, remove, clean"
  puts "  #{Colors::YELLOW}apt-fronted#{Colors::RESET} - APT frontend commands: install, remove, update, search"
end

def print_error(message)
  puts "#{Colors::RED}Error: #{message}#{Colors::RESET}"
end

def main
  if ARGV.empty?
    print_help
    exit(1)
  end

  home = ENV["HOME"]
  hpm_bin = "#{home}/.hackeros/hpm/hpm"
  community_bin = "#{home}/.hackeros/hpm/community"
  apt_frontend_bin = "#{home}/.hackeros/hpm/apt-fronted"
  tui_bin = "#{home}/.hackeros/hpm/tui"

  command = ARGV[0]
  args = ARGV[1..]

  case command
  when "install", "remove", "clean", "update"
    Process.run(hpm_bin, [command] + args, shell: false, output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  when "tui"
    Process.run(tui_bin, args, shell: false, output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  when "community"
    if args.empty?
      print_error("Community subcommand required: update, install, remove, clean")
      exit(1)
    end
    subcommand = args[0]
    subargs = args[1..]
    if ["update", "install", "remove", "clean"].includes?(subcommand)
      Process.run(community_bin, [subcommand] + subargs, shell: false, output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
    else
      print_error("Invalid community subcommand: #{subcommand}")
      exit(1)
    end
  when "apt-fronted"
    if args.empty?
      Process.run(apt_frontend_bin, args, shell: false, output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
    else
      subcommand = args[0]
      subargs = args[1..]
      if ["install", "remove", "update", "search"].includes?(subcommand)
        Process.run(apt_frontend_bin, [subcommand] + subargs, shell: false, output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
      else
        print_error("Invalid apt-fronted subcommand: #{subcommand}")
        exit(1)
      end
    end
  else
    print_error("Unknown command: #{command}")
    print_help
    exit(1)
  end
end

main
