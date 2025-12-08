import argparse
import sys
from display import colored, display_dnf_style, color_output, ProgressBar
from apt_utils import parse_apt_simulate, run_command

def confirm_action():
    while True:
        response = input(colored("Do you want to continue? [Y/n] ", 'yellow')).strip().lower()
        if response in ['', 'y', 'yes']:
            return True
        elif response in ['n', 'no']:
            return False
        else:
            print(colored("Please enter Y or N.", 'red'))

def handle_install(args):
    if not args.packages:
        print(colored("No packages specified for install.", 'red'))
        return
    cmd = ['sudo', 'apt', 'install', '-y'] + args.packages
    sim_cmd = ['sudo', 'apt', 'install'] + args.packages + ['-s']
    sim_output = run_command(sim_cmd, simulate=True, stream=False)
    parsed = parse_apt_simulate(sim_output)
    display_dnf_style(parsed, 'install')
    if confirm_action():
        print(colored("Running transaction", 'cyan'))
        run_command_with_progress(cmd)
    else:
        print(colored("Transaction cancelled.", 'yellow'))

def handle_remove(args):
    if not args.packages:
        print(colored("No packages specified for remove.", 'red'))
        return
    cmd = ['sudo', 'apt', 'remove', '-y'] + args.packages
    sim_cmd = ['sudo', 'apt', 'remove'] + args.packages + ['-s']
    sim_output = run_command(sim_cmd, simulate=True, stream=False)
    parsed = parse_apt_simulate(sim_output)
    display_dnf_style(parsed, 'remove')
    if confirm_action():
        print(colored("Running transaction", 'cyan'))
        run_command_with_progress(cmd)
    else:
        print(colored("Transaction cancelled.", 'yellow'))

def handle_update(args):
    print(colored("Updating package lists...", 'cyan'))
    update_cmd = ['sudo', 'apt', 'update']
    run_command_with_progress(update_cmd)

    upgrade_cmd = ['sudo', 'apt', 'upgrade', '-y']
    sim_cmd = ['sudo', 'apt', 'upgrade', '-s']
    sim_output = run_command(sim_cmd, simulate=True, stream=False)
    parsed = parse_apt_simulate(sim_output)
    display_dnf_style(parsed, 'upgrade')
    if confirm_action():
        print(colored("Running upgrade", 'cyan'))
        run_command_with_progress(upgrade_cmd)
    else:
        print(colored("Upgrade cancelled.", 'yellow'))

def handle_clean(args):
    autoclean_cmd = ['sudo', 'apt', 'autoclean']
    autoremove_cmd = ['sudo', 'apt', 'autoremove', '-y']
    sim_cmd = ['sudo', 'apt', 'autoremove', '-s']
    sim_output = run_command(sim_cmd, simulate=True, stream=False)
    parsed = parse_apt_simulate(sim_output)
    display_dnf_style(parsed, 'clean')
    if confirm_action():
        print(colored("Running autoclean", 'cyan'))
        run_command_with_progress(autoclean_cmd)
        print(colored("Running autoremove", 'cyan'))
        run_command_with_progress(autoremove_cmd)
    else:
        print(colored("Clean cancelled.", 'yellow'))

def run_command_with_progress(cmd):
    process = run_command(cmd, simulate=False, stream=False, return_process=True)
    progress = ProgressBar(total=100, desc=colored("Progress", 'blue'))
    output = ''
    while process.poll() is None:
        line = process.stdout.readline()
        if line:
            output += line
            sys.stdout.write(color_output(line))
            # Parse for progress
            if '%' in line:
                try:
                    percent = int(line.split('%')[0].strip().split()[-1])
                    progress.update(percent)
                except:
                    pass
    progress.close()
    remaining_output = process.stdout.read()
    if remaining_output:
        sys.stdout.write(color_output(remaining_output))
        output += remaining_output
    if process.returncode != 0:
        print(colored("Error executing command.", 'red'))
        sys.exit(1)
    return output

def main():
    parser = argparse.ArgumentParser(description=colored("Enhanced APT Frontend in DNF Style with Colors and Progress", 'magenta', bold=True))
    subparsers = parser.add_subparsers(dest='command')

    install_parser = subparsers.add_parser('install', help='Install packages')
    install_parser.add_argument('packages', nargs='*')
    install_parser.set_defaults(func=handle_install)

    remove_parser = subparsers.add_parser('remove', help='Remove packages')
    remove_parser.add_argument('packages', nargs='*')
    remove_parser.set_defaults(func=handle_remove)

    subparsers.add_parser('update', help='Update and upgrade packages').set_defaults(func=handle_update)
    subparsers.add_parser('clean', help='Clean up packages').set_defaults(func=handle_clean)

    args = parser.parse_args()
    if not args.command:
        parser.print_help()
        sys.exit(1)
    args.func(args)

if __name__ == "__main__":
    main()
