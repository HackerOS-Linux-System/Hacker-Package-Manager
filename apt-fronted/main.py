import argparse
import subprocess
import re
import sys

# Define ANSI color codes
class Colors:
    RESET = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'
    BLACK = '\033[30m'
    RED = '\033[31m'
    GREEN = '\033[32m'
    YELLOW = '\033[33m'
    BLUE = '\033[34m'
    MAGENTA = '\033[35m'
    CYAN = '\033[36m'
    WHITE = '\033[37m'

def colored(text, color, bold=False, underline=False):
    c = getattr(Colors, color.upper(), Colors.RESET)
    if bold:
        c += Colors.BOLD
    if underline:
        c += Colors.UNDERLINE
    return c + text + Colors.RESET

def parse_apt_simulate(output):
    installing = []
    upgrading = []
    removing = []
    download_size = '0'
    installed_size = '0'
    summary = (0, 0, 0)

    for line in output.splitlines():
        if line.startswith('Inst '):
            # Match Inst name [current_ver] (new_ver repo [arch])
            match = re.match(r'Inst (\S+) (?:\[(\S+)\] )?\((\S+) ([\S/]+) (?:(\S+))?\)', line)
            if match:
                name, current_ver, new_ver, repo, arch = match.groups()
                pkg = {'name': name, 'version': new_ver or current_ver, 'repo': repo, 'arch': arch or 'unknown'}
                if current_ver:
                    upgrading.append(pkg)
                else:
                    installing.append(pkg)
        elif line.startswith('Remv '):
            match = re.match(r'Remv (\S+) \[(\S+)\]', line)
            if match:
                name, ver = match.groups()
                removing.append({'name': name, 'version': ver, 'repo': 'N/A', 'arch': 'unknown'})
        elif 'Need to get' in line:
            match = re.search(r'Need to get ([\d.,]+ [kMG]?B) of archives.', line)
            if match:
                download_size = match.group(1)
        elif 'After this operation' in line:
            match = re.search(r'After this operation, ([\d.,]+ [kMG]?B) (?:of additional disk space will be used|disk space will be freed).', line)
            if match:
                installed_size = match.group(1)
        elif re.match(r'\d+ upgraded, \d+ newly installed, \d+ to remove and \d+ not upgraded.', line):
            match = re.match(r'(\d+) upgraded, (\d+) newly installed, (\d+) to remove and (\d+) not upgraded.', line)
            if match:
                summary = (int(match.group(2)), int(match.group(1)), int(match.group(3)))

    return {
        'installing': installing,
        'upgrading': upgrading,
        'removing': removing,
        'download_size': download_size,
        'installed_size': installed_size,
        'summary': summary
    }

def display_dnf_style(parsed, action='install'):
    print(colored("Dependencies resolved.", 'cyan'))
    print("=" * 80)
    print(f" {colored('Package', 'bold'):<30} {colored('Arch', 'bold'):<10} {colored('Version', 'bold'):<20} {colored('Repository', 'bold'):<20}")
    print("=" * 80)

    if parsed['installing']:
        print(colored("Installing:", 'green', bold=True))
        for pkg in parsed['installing']:
            print(f" {pkg['name']:<30} {pkg['arch']:<10} {pkg['version']:<20} {pkg['repo']:<20}")

    if parsed['upgrading']:
        print(colored("Upgrading:", 'blue', bold=True))
        for pkg in parsed['upgrading']:
            print(f" {pkg['name']:<30} {pkg['arch']:<10} {pkg['version']:<20} {pkg['repo']:<20}")

    if parsed['removing']:
        print(colored("Removing:", 'red', bold=True))
        for pkg in parsed['removing']:
            print(f" {pkg['name']:<30} {pkg['arch']:<10} {pkg['version']:<20} {pkg['repo']:<20}")

    print("\n" + colored("Transaction Summary", 'cyan'))
    print("=" * 80)
    print(f"Install   {parsed['summary'][0]} Packages")
    print(f"Upgrade   {parsed['summary'][1]} Packages")
    print(f"Remove    {parsed['summary'][2]} Packages")
    print(f"\nTotal download size: {parsed['download_size']}")
    if action in ['install', 'upgrade']:
        print(f"Installed size: {parsed['installed_size']}")
    elif action == 'remove':
        print(f"Freed size: {parsed['installed_size']}")
    print()

def run_command(cmd, simulate=False, stream=True):
    if simulate:
        cmd += ['-s']
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, shell=False)
    output = ''
    if stream:
        for line in process.stdout:
            sys.stdout.write(color_output(line))
            output += line
    else:
        output = process.communicate()[0]
    process.wait()
    if process.returncode != 0:
        print(colored("Error executing command.", 'red'))
        sys.exit(1)
    return output

def color_output(line):
    if 'Setting up' in line or 'Installing' in line:
        return colored(line, 'green')
    elif 'Removing' in line:
        return colored(line, 'red')
    elif 'Unpacking' in line:
        return colored(line, 'yellow')
    elif 'Reading' in line or 'Building' in line:
        return colored(line, 'cyan')
    else:
        return line

def handle_install(args):
    if not args.packages:
        print("No packages specified for install.")
        return
    cmd = ['sudo', 'apt', 'install', '-y'] + args.packages
    sim_output = run_command(cmd, simulate=True, stream=False)
    parsed = parse_apt_simulate(sim_output)
    display_dnf_style(parsed, 'install')
    print(colored("Running transaction", 'cyan'))
    run_command(cmd, simulate=False, stream=True)

def handle_remove(args):
    if not args.packages:
        print("No packages specified for remove.")
        return
    cmd = ['sudo', 'apt', 'remove', '-y'] + args.packages
    sim_output = run_command(cmd, simulate=True, stream=False)
    parsed = parse_apt_simulate(sim_output)
    display_dnf_style(parsed, 'remove')
    print(colored("Running transaction", 'cyan'))
    run_command(cmd, simulate=False, stream=True)

def handle_update(args):
    # First, apt update
    print(colored("Updating package lists...", 'cyan'))
    update_cmd = ['sudo', 'apt', 'update']
    run_command(update_cmd, simulate=False, stream=True)
    
    # Then, simulate upgrade
    upgrade_cmd = ['sudo', 'apt', 'upgrade', '-y']
    sim_output = run_command(upgrade_cmd, simulate=True, stream=False)
    parsed = parse_apt_simulate(sim_output)
    display_dnf_style(parsed, 'upgrade')
    print(colored("Running transaction", 'cyan'))
    run_command(upgrade_cmd, simulate=False, stream=True)

def handle_clean(args):
    # Simulate autoremove
    autoremove_cmd = ['sudo', 'apt', 'autoremove', '-y']
    sim_output = run_command(autoremove_cmd, simulate=True, stream=False)
    parsed = parse_apt_simulate(sim_output)
    display_dnf_style(parsed, 'clean')
    
    # Run autoclean
    print(colored("Running autoclean", 'cyan'))
    autoclean_cmd = ['sudo', 'apt', 'autoclean']
    run_command(autoclean_cmd, simulate=False, stream=True)
    
    # Run autoremove
    print(colored("Running autoremove", 'cyan'))
    run_command(autoremove_cmd, simulate=False, stream=True)

def main():
    parser = argparse.ArgumentParser(description="APT frontend in DNF style with colors.")
    subparsers = parser.add_subparsers(dest='command')

    install_parser = subparsers.add_parser('install')
    install_parser.add_argument('packages', nargs='*')
    install_parser.set_defaults(func=handle_install)

    remove_parser = subparsers.add_parser('remove')
    remove_parser.add_argument('packages', nargs='*')
    remove_parser.set_defaults(func=handle_remove)

    subparsers.add_parser('update').set_defaults(func=handle_update)
    subparsers.add_parser('clean').set_defaults(func=handle_clean)

    args = parser.parse_args()
    if not args.command:
        parser.print_help()
        sys.exit(1)
    args.func(args)

if __name__ == "__main__":
    main()
