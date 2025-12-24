import subprocess
import sys
import re
import time
from tqdm import tqdm
import colorama
from colorama import Fore, Style
from termcolor import colored
import blessed
from rich.console import Console
from rich.progress import Progress, BarColumn, TextColumn, TimeRemainingColumn
from rich.panel import Panel
from rich.prompt import Prompt
from rich.style import Style as RichStyle
# Initialize colorama for cross-platform ANSI support
colorama.init()
# Rich console for advanced output
console = Console()
# Blessed terminal for some cursor control if needed
term = blessed.Terminal()
def print_header():
    """Print a fancy header for the tool."""
    console.print(Panel.fit(
        "[bold cyan]APT-Fronted[/bold cyan] - A Beautiful Frontend for APT",
        border_style="bold magenta",
        title="Welcome to APT-Fronted",
        subtitle="Powered by HackerOS"
    ))
def run_command(cmd, capture_output=False, check=True):
    """Run a subprocess command with error handling."""
    try:
        if capture_output:
            return subprocess.run(cmd, capture_output=True, text=True, check=check)
        else:
            subprocess.run(cmd, check=check)
    except subprocess.CalledProcessError as e:
        console.print(f"[bold red]Error running command:[/bold red] {e.cmd}")
        if e.stderr:
            console.print(f"[red]{e.stderr.strip()}[/red]")
        sys.exit(1)
def parse_apt_output(output):
    """Parse APT output for progress indicators."""
    # Simple regex to find progress like "Reading package lists... Done"
    # Or download progress
    progress_matches = re.findall(r'(\d+)%', output)
    if progress_matches:
        return int(progress_matches[-1])
    return 0
def simulate_progress(task_name, steps=100):
    """Simulate a progress bar for operations without real-time output."""
    with Progress(
        TextColumn("[bold blue]{task.description}", justify="right"),
        BarColumn(bar_width=None),
        "[progress.percentage]{task.percentage:>3.1f}%",
        TimeRemainingColumn(),
        console=console
    ) as progress:
        task = progress.add_task(task_name, total=steps)
        for _ in range(steps):
            time.sleep(0.05) # Simulate work
            progress.update(task, advance=1)
def apt_update():
    """Run apt update with progress."""
    console.print(f"[green]Running {colored('apt update', 'yellow', attrs=['bold'])}...[/green]")
    try:
        # Run apt update with subprocess and capture output for parsing
        proc = subprocess.Popen(['sudo', 'apt', 'update'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
       
        with Progress(
            TextColumn("[bold green]Updating...[/bold green]"),
            BarColumn(),
            "[progress.percentage]{task.percentage:>3.1f}%",
            console=console
        ) as progress:
            task = progress.add_task("Update Progress", total=100)
            current_progress = 0
            while proc.poll() is None:
                line = proc.stdout.readline().strip()
                if line:
                    console.print(f"[dim]{line}[/dim]")
                new_progress = parse_apt_output(line)
                if new_progress > current_progress:
                    progress.update(task, completed=new_progress)
                    current_progress = new_progress
                time.sleep(0.1)
       
        if proc.returncode != 0:
            error = proc.stderr.read().strip()
            console.print(f"[bold red]Update failed:[/bold red] {error}")
        else:
            console.print("[bold green]Update completed successfully![/bold green]")
    except Exception as e:
        console.print(f"[bold red]Error during update:[/bold red] {str(e)}")
def apt_install(package):
    """Install a package with progress and error handling."""
    console.print(f"[green]Installing {colored(package, 'cyan', attrs=['bold'])}...[/green]")
   
    # Check if package is already installed
    check_cmd = ['dpkg', '-s', package]
    check = subprocess.run(check_cmd, capture_output=True, text=True)
    if check.returncode == 0:
        console.print(f"[yellow]{package} is already installed.[/yellow]")
        return
   
    try:
        # Run apt install
        proc = subprocess.Popen(['sudo', 'apt', 'install', '-y', package], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
       
        with Progress(
            TextColumn("[bold cyan]Installing...[/bold cyan]"),
            BarColumn(),
            "[progress.percentage]{task.percentage:>3.1f}%",
            console=console
        ) as progress:
            task = progress.add_task("Install Progress", total=100)
            current_progress = 0
            while proc.poll() is None:
                line = proc.stdout.readline().strip()
                if line:
                    console.print(f"[dim]{line}[/dim]")
                new_progress = parse_apt_output(line)
                if new_progress > current_progress:
                    progress.update(task, completed=new_progress)
                    current_progress = new_progress
                time.sleep(0.1)
       
        if proc.returncode != 0:
            error = proc.stderr.read().strip()
            if "unable to locate package" in error.lower():
                console.print(f"[bold red]Package {package} not found![/bold red]")
            elif "has no installation candidate" in error.lower():
                console.print(f"[bold red]No installation candidate for {package}.[/bold red]")
            else:
                console.print(f"[bold red]Installation failed:[/bold red] {error}")
        else:
            console.print(f"[bold green]{package} installed successfully![/bold green]")
    except Exception as e:
        console.print(f"[bold red]Error during installation:[/bold red] {str(e)}")
def apt_remove(package):
    """Remove a package with confirmation."""
    confirm = Prompt.ask(f"[yellow]Are you sure you want to remove {package}? (y/n)[/yellow]", default="n")
    if confirm.lower() != 'y':
        console.print("[blue]Removal cancelled.[/blue]")
        return
   
    console.print(f"[green]Removing {colored(package, 'red', attrs=['bold'])}...[/green]")
    simulate_progress("Removing package", steps=50) # Simulated for removal
    run_command(['sudo', 'apt', 'remove', '-y', package])
    console.print(f"[bold green]{package} removed successfully![/bold green]")
def apt_search(query):
    """Search for packages."""
    console.print(f"[green]Searching for {colored(query, 'magenta', attrs=['bold'])}...[/green]")
    result = run_command(['apt', 'search', query], capture_output=True)
    output = result.stdout.strip()
    if output:
        console.print(Panel(output, title="Search Results", border_style="green"))
    else:
        console.print("[yellow]No results found.[/yellow]")
def main():
    print_header()
   
    if len(sys.argv) < 2:
        console.print("[bold red]Usage:[/bold red] apt-fronted [update|install <pkg>|remove <pkg>|search <query>]")
        sys.exit(1)
   
    command = sys.argv[1].lower()
   
    if command == 'update':
        apt_update()
    elif command == 'install':
        if len(sys.argv) < 3:
            console.print("[bold red]Please provide a package name.[/bold red]")
            sys.exit(1)
        apt_install(sys.argv[2])
    elif command == 'remove':
        if len(sys.argv) < 3:
            console.print("[bold red]Please provide a package name.[/bold red]")
            sys.exit(1)
        apt_remove(sys.argv[2])
    elif command == 'search':
        if len(sys.argv) < 3:
            console.print("[bold red]Please provide a search query.[/bold red]")
            sys.exit(1)
        apt_search(sys.argv[2])
    else:
        console.print(f"[bold red]Unknown command: {command}[/bold red]")
if __name__ == "__main__":
    main()
