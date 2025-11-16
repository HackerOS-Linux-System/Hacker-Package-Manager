from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from rich.tree import Tree
from prompt_toolkit import PromptSession
from prompt_toolkit.completion import WordCompleter
from prompt_toolkit.styles import Style
import subprocess
import sys
import os

console = Console()

commands = [
    "unpack add-ons", "unpack gs", "unpack devtools", "unpack emulators", "unpack cybersecurity",
    "unpack select", "unpack gaming", "unpack noroblox", "unpack hacker-mode", "unpack gamescope-session-steam",
    "help", "docs", "install", "remove",
    "flatpak-install", "flatpak-remove", "flatpak-update",
    "system logs",
    "run update-system", "run check-updates", "run steam", "run hacker-launcher", "run hackeros-game-mode", "run update-hackeros",
    "update", "game", "hacker-lang", "ascii", "shell", "enter", "remove-container", "exit"
]

completer = WordCompleter(commands, ignore_case=True)

style = Style.from_dict({
    'prompt': 'blue bold',
})

def display_command_list():
    console.clear()
    title = Text("Hacker Shell - Commands (Expanded)", style="bold purple")
    tree = Tree("Available Commands", style="bold blue")
    unpack = tree.add("unpack: Unpack various toolsets", style="blue")
    unpack.add("add-ons, gs, devtools, emulators, cybersecurity (container), select (TUI), gaming, noroblox, hacker-mode, gamescope-session-steam")
    tree.add("help: Display help (TUI)")
    tree.add("docs: Display documentation and FAQ (TUI)")
    tree.add("install <package>: APT install")
    tree.add("remove <package>: APT remove")
    tree.add("flatpak-install <package>")
    tree.add("flatpak-remove <package>")
    tree.add("flatpak-update")
    system = tree.add("system: System commands", style="blue")
    system.add("logs")
    run = tree.add("run: Run scripts", style="blue")
    run.add("update-system, check-updates, steam, hacker-launcher, hackeros-game-mode, update-hackeros")
    tree.add("update: Full system update")
    tree.add("game: Play expanded Hacker Adventure")
    tree.add("hacker-lang: Info on Hacker lang")
    tree.add("ascii: Show ASCII art")
    tree.add("shell: Enter shell (recursive)")
    tree.add("enter <container>: Enter distrobox container")
    tree.add("remove-container <container>: Remove distrobox container")
    tree.add("exit: Exit the shell")
    panel = Panel(tree, title=title, expand=False, border_style="purple")
    console.print(panel)

def run_hacker_command(cmd):
    if cmd.strip() == "":
        return
    if cmd == "exit":
        console.print("Exiting Hacker Shell...", style="gray")
        sys.exit(0)
    console.print(f"Executing: hacker {cmd}", style="purple")
    try:
        process = subprocess.Popen(["hacker"] + cmd.split(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        stdout, stderr = process.communicate()
        if process.returncode == 0:
            if stdout.strip():
                console.print(stdout, style="white")
            else:
                console.print("Command executed successfully (no output).", style="gray")
        else:
            console.print(stderr, style="red")
            console.print(f"Error: Command failed with exit code {process.returncode}", style="red")
    except Exception as e:
        console.print(f"Error executing command: {e}", style="red")

def main():
    session = PromptSession(completer=completer, style=style)
    console.print("Welcome to Hacker Shell! Type 'exit' to quit.", style="blue")
    while True:
        display_command_list()
        try:
            cmd = session.prompt('hacker-shell> ')
            run_hacker_command(cmd)
            console.input("Press Enter to continue...")
        except KeyboardInterrupt:
            console.print("\nInterrupted. Type 'exit' to quit.", style="gray")
            continue
        except EOFError:
            break

if __name__ == "__main__":
    main()
