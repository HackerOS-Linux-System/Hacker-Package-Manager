import subprocess
import re
import sys

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

    lines = output.splitlines()
    for i, line in enumerate(lines):
        if line.startswith('Inst '):
            match = re.match(r'Inst (\S+) (?:\[(\S+)\] )?\((\S+) ([\S/]+) (?:\[(\S+)\])?\)', line)
            if match:
                name, current_ver, new_ver, repo, arch = match.groups()
                arch = arch or 'unknown'
                pkg = {'name': name, 'version': new_ver or current_ver, 'repo': repo, 'arch': arch}
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
        elif re.match(r'\d+ (?:packages? )?upgraded, \d+ newly installed, \d+ to remove and \d+ not upgraded.', line):
            match = re.match(r'(\d+) (?:packages? )?upgraded, (\d+) newly installed, (\d+) to remove and (\d+) not upgraded.', line)
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

def run_command(cmd, simulate=False, stream=True, return_process=False):
    if simulate:
        cmd += ['-qq']  # quieter for simulation
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1, universal_newlines=True)
    if return_process:
        return process
    output = ''
    if stream:
        for line in iter(process.stdout.readline, ''):
            sys.stdout.write(line)
            output += line
    else:
        output, _ = process.communicate()
    process.wait()
    if process.returncode != 0:
        print(colored(f"Error executing {' '.join(cmd)}", 'red'))
    return output
