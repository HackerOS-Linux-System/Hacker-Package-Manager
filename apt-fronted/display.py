from apt_utils import Colors, colored
from tqdm import tqdm

class ProgressBar(tqdm):
    def __init__(self, *args, **kwargs):
        kwargs['bar_format'] = '{desc}: {percentage:3.0f}%|{bar}| {n_fmt}/{total_fmt} [{elapsed}<{remaining}, {rate_fmt}{postfix}]'
        super().__init__(*args, **kwargs)

def display_dnf_style(parsed, action='install'):
    print(colored("\nDependencies resolved.", 'cyan', bold=True))
    print(colored("=" * 90, 'white'))
    print(f" {colored('Package', 'yellow', bold=True):<35} {colored('Arch', 'yellow', bold=True):<12} {colored('Version', 'yellow', bold=True):<25} {colored('Repository', 'yellow', bold=True):<20}")
    print(colored("=" * 90, 'white'))

    if parsed['installing']:
        print(colored("Installing:", 'green', bold=True))
        for pkg in parsed['installing']:
            print(f" {colored(pkg['name'], 'green'):<35} {pkg['arch']:<12} {pkg['version']:<25} {pkg['repo']:<20}")

    if parsed['upgrading']:
        print(colored("Upgrading:", 'blue', bold=True))
        for pkg in parsed['upgrading']:
            print(f" {colored(pkg['name'], 'blue'):<35} {pkg['arch']:<12} {pkg['version']:<25} {pkg['repo']:<20}")

    if parsed['removing']:
        print(colored("Removing:", 'red', bold=True))
        for pkg in parsed['removing']:
            print(f" {colored(pkg['name'], 'red'):<35} {pkg['arch']:<12} {pkg['version']:<25} {pkg['repo']:<20}")

    print("\n" + colored("Transaction Summary", 'cyan', bold=True))
    print(colored("=" * 90, 'white'))
    print(f"{colored('Install', 'green')}   {parsed['summary'][0]} Packages")
    print(f"{colored('Upgrade', 'blue')}   {parsed['summary'][1]} Packages")
    print(f"{colored('Remove', 'red')}    {parsed['summary'][2]} Packages")
    print(f"\n{colored('Total download size:', 'magenta')} {parsed['download_size']}")
    if action in ['install', 'upgrade']:
        print(f"{colored('Installed size:', 'magenta')} {parsed['installed_size']}")
    elif action == 'remove':
        print(f"{colored('Freed size:', 'magenta')} {parsed['installed_size']}")
    print(colored("=" * 90, 'white') + "\n")

def color_output(line):
    line = line.strip()
    if 'Setting up' in line or 'Installing' in line or 'Unpacking' in line:
        return colored(line, 'green') + '\n'
    elif 'Removing' in line:
        return colored(line, 'red') + '\n'
    elif 'Downloading' in line or 'Get:' in line:
        return colored(line, 'yellow') + '\n'
    elif 'Reading' in line or 'Building' in line:
        return colored(line, 'cyan') + '\n'
    elif 'Hit:' in line or 'Ign:' in line:
        return colored(line, 'white') + '\n'
    else:
        return line + '\n'
