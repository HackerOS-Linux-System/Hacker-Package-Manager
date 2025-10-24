package main

import (
	"fmt"
	"os"

	"github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/bubbles/list"
	"github.com/charmbracelet/bubbles/viewport"
	"github.com/charmbracelet/lipgloss"
)

type Command struct {
	Name        string
	Description string
	Details     string
}

var commands = []Command{
	{
		Name:        "hacker unpack add-ons",
		Description: "Install Wine, BoxBuddy, Winezgui, Gearlever",
		Details:     "This command installs add-ons like Wine for running Windows applications, BoxBuddy for managing Flatpaks, Winezgui for Wine GUI, and Gearlever for additional utilities.",
	},
	{
		Name:        "hacker unpack g-s",
		Description: "Install gaming and cybersecurity tools",
		Details:     "Installs both gaming (Steam, Lutris, etc.) and cybersecurity tools (nmap, wireshark, etc.) in one go.",
	},
	{
		Name:        "hacker unpack devtools",
		Description: "Install Atom",
		Details:     "Installs the Atom text editor via Flatpak for development purposes.",
	},
	{
		Name:        "hacker unpack emulators",
		Description: "Install PlayStation, Nintendo, DOSBox, PS3 emulators",
		Details:     "Installs various emulators including shadPS4, Ryujinx, DOSBox-X, and RPCS3.",
	},
	{
		Name:        "hacker unpack cybersecurity",
		Description: "Install nmap, wireshark, Metasploit, Ghidra, etc.",
		Details:     "Installs a suite of cybersecurity tools for penetration testing, including nmap, wireshark, nikto, Metasploit, Ghidra, and more.",
	},
	{
		Name:        "hacker unpack hacker-mode",
		Description: "Install gamescope",
		Details:     "Installs gamescope for advanced gaming session management.",
	},
	{
		Name:        "hacker unpack select",
		Description: "Interactive package selection",
		Details:     "Provides an interactive CLI menu to select and install specific package groups.",
	},
	{
		Name:        "hacker unpack gaming",
		Description: "Install OBS Studio, Lutris, Steam, etc.",
		Details:     "Installs gaming tools including OBS Studio, Lutris, Steam, Heroic Games Launcher, Discord, and Roblox support.",
	},
	{
		Name:        "hacker unpack noroblox",
		Description: "Install gaming tools without Roblox",
		Details:     "Installs gaming tools like the gaming command but excludes Roblox-related packages.",
	},
	{
		Name:        "hacker help",
		Description: "Display this help message",
		Details:     "Launches this interactive help UI.",
	},
	{
		Name:        "hacker install <package>",
		Description: "Placeholder for installing packages",
		Details:     "Currently a placeholder; will print a message about the package.",
	},
	{
		Name:        "hacker remove <package>",
		Description: "Placeholder for removing packages",
		Details:     "Currently a placeholder; will print a message about the package.",
	},
	{
		Name:        "hacker apt-install <package>",
		Description: "Run apt install -y <package>",
		Details:     "Uses sudo apt install -y to install the specified Debian package.",
	},
	{
		Name:        "hacker apt-remove <package>",
		Description: "Run apt remove -y <package>",
		Details:     "Uses sudo apt remove -y to remove the specified Debian package.",
	},
	{
		Name:        "hacker flatpak-install <package>",
		Description: "Run flatpak install -y flathub <package>",
		Details:     "Installs a Flatpak package from Flathub.",
	},
	{
		Name:        "hacker flatpak-remove <package>",
		Description: "Run flatpak remove -y <package>",
		Details:     "Removes a Flatpak package.",
	},
	{
		Name:        "hacker flatpak-update",
		Description: "Run flatpak update -y",
		Details:     "Updates all installed Flatpak packages.",
	},
	{
		Name:        "hacker system logs",
		Description: "Show system logs",
		Details:     "Displays recent system logs using journalctl -xe.",
	},
	{
		Name:        "hacker run hackeros-cockpit",
		Description: "Run HackerOS Cockpit",
		Details:     "Launches the HackerOS Cockpit Python script.",
	},
	{
		Name:        "hacker run switch-to-other-session",
		Description: "Switch to another session",
		Details:     "Runs a script to switch to another desktop session.",
	},
	{
		Name:        "hacker run update-system",
		Description: "Update the system",
		Details:     "Runs the system update script.",
	},
	{
		Name:        "hacker run check-updates",
		Description: "Check for system updates",
		Details:     "Runs the update check notification script.",
	},
	{
		Name:        "hacker run steam",
		Description: "Launch Steam via HackerOS script",
		Details:     "Launches Steam using a custom HackerOS script.",
	},
	{
		Name:        "hacker run hacker-launcher",
		Description: "Launch HackerOS Launcher",
		Details:     "Runs the HackerOS application launcher.",
	},
	{
		Name:        "hacker run hackeros-game-mode",
		Description: "Run HackerOS Game Mode",
		Details:     "Launches the HackerOS Game Mode AppImage.",
	},
	{
		Name:        "hacker update",
		Description: "Perform system update (apt, flatpak, snap, firmware, omz)",
		Details:     "Updates APT, Flatpak, Snap, firmware, and Oh-My-Zsh.",
	},
	{
		Name:        "hacker game",
		Description: "Play a fun Hacker Adventure game",
		Details:     "Starts an interactive text-based adventure game in the terminal.",
	},
	{
		Name:        "hacker hacker-lang",
		Description: "Information about Hacker programming language",
		Details:     "Displays info about using the .hacker file extension and hackerc compiler.",
	},
	{
		Name:        "hacker ascii",
		Description: "Display HackerOS ASCII art",
		Details:     "Shows the HackerOS ASCII art from the config file.",
	},
}

type item struct {
	cmd Command
}

func (i item) Title() string       { return i.cmd.Name }
func (i item) Description() string { return i.cmd.Description }
func (i item) FilterValue() string { return i.cmd.Name + " " + i.cmd.Description }

type mode int

const (
	listMode mode = iota
	detailsMode
)

type keyMap struct {
	quit key.Binding
}

func newKeyMap() *keyMap {
	return &keyMap{
		quit: key.NewBinding(
			key.WithKeys("q", "ctrl+c"),
				     key.WithHelp("q", "quit"),
		),
	}
}

type model struct {
	list     list.Model
	viewport viewport.Model
	keys     *keyMap
	ready    bool
	mode     mode
	selected int
}

func newModel() model {
	var items []list.Item
	items = append(items, item{Command{Name: "Exit", Description: "Press to exit"}})
	for _, c := range commands {
		items = append(items, item{c})
	}

	delegate := list.NewDefaultDelegate()
	delegate.Styles.SelectedTitle = delegate.Styles.SelectedTitle.Foreground(lipgloss.Color("#FF75CB")).Bold(true)

	l := list.New(items, delegate, 0, 0)
	l.Title = "Commands"
	l.SetShowStatusBar(false)
	l.SetFilteringEnabled(true)
	l.Styles.Title = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#FA8072")).Padding(0, 1)
	l.SetShowHelp(true)

	vp := viewport.New(0, 0)
	vp.Style = lipgloss.NewStyle().Border(lipgloss.NormalBorder(), true).Padding(1)

	return model{
		list:     l,
		viewport: vp,
		keys:     newKeyMap(),
	}
}

func (m model) Init() tea.Cmd {
	return nil
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd

	switch msg := msg.(type) {
		case tea.WindowSizeMsg:
			m.list.SetWidth(msg.Width / 2 - 1)
			m.list.SetHeight(msg.Height - 2)
			m.viewport.Width = msg.Width / 2 - 4 // account for padding and border
			m.viewport.Height = msg.Height - 4
			if !m.ready {
				m.ready = true
			}
			return m, nil

		case tea.KeyMsg:
			if key.Matches(msg, m.keys.quit) {
				return m, tea.Quit
			}

			switch msg.String() {
				case "enter":
					if m.mode == listMode {
						idx := m.list.Index()
						if idx == 0 {
							return m, tea.Quit
						}
						m.selected = idx - 1
						m.viewport.SetContent(commands[m.selected].Details)
						m.viewport.GotoTop()
						m.mode = detailsMode
						return m, nil
					}
				case "esc":
					if m.mode == detailsMode {
						m.mode = listMode
						return m, nil
					}
			}
	}

	if m.mode == listMode {
		m.list, cmd = m.list.Update(msg)
	} else {
		m.viewport, cmd = m.viewport.Update(msg)
	}

	return m, cmd
}

func (m model) View() string {
	if !m.ready {
		return "Initializing..."
	}

	left := lipgloss.NewStyle().
	Width(m.list.Width() + 2).
	Border(lipgloss.NormalBorder(), false, true, false, false).
	Render(m.list.View())

	rightContent := "Select a command from the list to view details.\n\nPress 'enter' to select, 'esc' to go back, 'q' to quit."
	if m.mode == detailsMode {
		rightContent = m.viewport.View()
	}

	right := lipgloss.NewStyle().
	Width(m.viewport.Width + 4). // account for padding and border
	Height(m.list.Height() + 2).
	Border(lipgloss.NormalBorder(), true).
	Padding(1).
	Render(rightContent)

	return lipgloss.JoinHorizontal(lipgloss.Top, left, right)
}

func main() {
	p := tea.NewProgram(newModel(), tea.WithAltScreen())
	if _, err := p.Run(); err != nil {
		fmt.Printf("Error: %v\n", err)
		os.Exit(1)
	}
}
