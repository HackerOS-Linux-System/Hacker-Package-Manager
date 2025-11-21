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
		Name:        "hacker unpack xanmod",
		Description: "Install the xanmod kernel.",
		Details:     "Note: After restarting the system, there will be no default kernel, you will only have the xanmod kernel.",
	},
	{
		Name:        "hacker unpack liquorix",
		Description: "Install liquorix kernel",
		Details:     "Note: After restarting the system, there will be no default kernel, you will only have the liquorix kernel.",
	},
	{
		Name:        "hacker unpack add-ons",
		Description: "Install Wine, BoxBuddy, Winezgui, Gearlever",
		Details:     "This command installs add-ons like Wine for running Windows applications, BoxBuddy for managing Flatpaks, Winezgui for Wine GUI, and Gearlever for additional utilities.",
	},
	{
		Name:        "hacker unpack g-s",
		Description: "Install gaming and cybersecurity tools",
		Details:     "Installs both gaming (Steam, Lutris, etc.) and cybersecurity tools (container with BlackArch).",
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
		Description: "Setup cybersecurity container with BlackArch",
		Details:     "Creates a distrobox container with Arch Linux, installs BlackArch, enables multilib, and allows selecting categories or all tools.",
	},
	{
		Name:        "hacker unpack hacker-mode",
		Description: "Install gamescope",
		Details:     "Installs gamescope for advanced gaming session management.",
	},
	{
		Name:        "hacker unpack select",
		Description: "Interactive package selection with TUI",
		Details:     "Provides an interactive TUI to select categories or individual applications, with search capability.",
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
		Name:        "hacker unpack gamescope-session-steam",
		Description: "Install and setup gamescope-session-steam",
		Details:     "Checks and installs gamescope via apt, checks and installs Steam flatpak, clones the repo to /tmp, and runs unpack.hacker with hackerc.",
	},
	{
		Name:        "hacker help",
		Description: "Display this help message",
		Details:     "Launches this interactive help UI.",
	},
	{
		Name:        "hacker docs",
		Description: "Display documentation and FAQ",
		Details:     "Launches an interactive UI with frequently asked questions and answers for beginners.",
	},
	{
		Name:        "hacker install <package>",
		Description: "Install package using apt",
		Details:     "Runs sudo apt install -y <package>.",
	},
	{
		Name:        "hacker remove <package>",
		Description: "Remove package using apt",
		Details:     "Runs sudo apt remove -y <package>.",
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
		Name:        "hacker run update-hackeros",
		Description: "Update HackerOS",
		Details:     "Runs the update-hackeros.sh script to update HackerOS.",
	},
	{
		Name:        "hacker update",
		Description: "Perform system update (apt, flatpak, snap, firmware, omz)",
		Details:     "Updates APT, Flatpak, Snap, firmware, and Oh-My-Zsh.",
	},
	{
		Name:        "hacker game",
		Description: "Play a fun Hacker Adventure game",
		Details:     "Starts an interactive text-based adventure game in the terminal with expanded levels and challenges.",
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
	{
		Name:        "hacker enter <container>",
		Description: "Enter a distrobox container",
		Details:     "Runs distrobox enter <container> to access the container shell.",
	},
	{
		Name:        "hacker remove-container <container>",
		Description: "Remove a distrobox container",
		Details:     "Stops and removes the specified distrobox container after confirmation.",
	},
	{
		Name:        "hacker plugin create <name>",
		Description: "Create a new plugin template",
		Details:     "Create new plugin in .yaml.",
	},
	{
		Name:        "hacker plugin enable <name>",
		Description: "Enable a plugin",
		Details:     "Enable plugins.",
	},
	{
		Name:        "hacker plugin disable <name>",
		Description: "Disable a plugin",
		Details:     "Disable plugins.",
	},
	{
		Name:        "hacker plugin list",
		Description: "List available and enabled plugins",
		Details:     "List for every plugin.",
	},
	{
		Name:        "hacker plugin apply",
		Description: "Apply all enabled plugins (run their commands)",
		Details:     "Runs plugins.",
	},
	{
		Name:        "hacker restart <service>",
		Description: "Restart custom systemctl service",
		Details:     "Restart services.",
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
