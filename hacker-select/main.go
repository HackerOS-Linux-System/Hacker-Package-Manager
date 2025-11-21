package main

import (
	"flag"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"

	"github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/bubbles/list"
	"github.com/charmbracelet/lipgloss"
)

type itemType string

const (
	header   itemType = "header"
	category itemType = "category"
	app      itemType = "app"
)

type item struct {
	typ      itemType
	title    string
	desc     string
	value    string
	selected bool
}

func (i item) Title() string       { return i.title }
func (i item) Description() string { return i.desc }
func (i item) FilterValue() string { return i.title + " " + i.desc }

type keyMap struct {
	quit   key.Binding
	toggle key.Binding
}

func newKeyMap() *keyMap {
	return &keyMap{
		quit: key.NewBinding(
			key.WithKeys("q", "ctrl+c"),
				     key.WithHelp("q", "quit"),
		),
		toggle: key.NewBinding(
			key.WithKeys(" "),
				       key.WithHelp("space", "toggle select"),
		),
	}
}

type model struct {
	list  list.Model
	keys  *keyMap
	ready bool
	mode  string // to know how to print
}

func newModel(items []list.Item, mode string) model {
	delegate := list.NewDefaultDelegate()
	delegate.Styles.SelectedTitle = delegate.Styles.SelectedTitle.Foreground(lipgloss.Color("#FF75CB")).Bold(true)
	l := list.New(items, delegate, 0, 0)
	l.Title = "Select Items (use space to toggle, enter to confirm)"
	l.SetShowStatusBar(false)
	l.SetFilteringEnabled(true)
	l.Styles.Title = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#FA8072")).Padding(0, 1)
	l.SetShowHelp(true)
	del := &customDelegate{DefaultDelegate: delegate}
	l.SetDelegate(del)
	return model{
		list:  l,
		keys:  newKeyMap(),
		mode:  mode,
	}
}

type customDelegate struct {
	list.DefaultDelegate
}

func (d customDelegate) Render(w io.Writer, m list.Model, index int, listItem list.Item) {
	i, ok := listItem.(item)
	if !ok {
		return
	}
	if i.typ == header {
		fmt.Fprint(w, lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#00FF00")).Render(i.title))
		return
	}
	checkbox := "[ ] "
	if i.selected {
		checkbox = "[x] "
	}
	str := checkbox + i.title
	if i.desc != "" {
		str += "\n  " + lipgloss.NewStyle().Foreground(lipgloss.Color("#AAAAAA")).Render(i.desc)
	}
	fn := d.Styles.NormalTitle.Render
	if index == m.Index() {
		fn = func(s ...string) string {
			return d.Styles.SelectedTitle.Render(s...)
		}
	}
	fmt.Fprint(w, fn(str))
}

func (m model) Init() tea.Cmd {
	return nil
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
		case tea.WindowSizeMsg:
			m.list.SetWidth(msg.Width)
			m.list.SetHeight(msg.Height - 2)
			if !m.ready {
				m.ready = true
			}
			return m, nil
		case tea.KeyMsg:
			if key.Matches(msg, m.keys.quit) {
				return m, tea.Quit
			}
			if key.Matches(msg, m.keys.toggle) {
				idx := m.list.Index()
				currItem, ok := m.list.Items()[idx].(item)
				if ok && currItem.typ != header {
					currItem.selected = !currItem.selected
					m.list.SetItem(idx, currItem)
				}
				return m, nil
			}
			if msg.String() == "enter" {
				for _, it := range m.list.Items() {
					i, ok := it.(item)
					if ok && i.selected {
						if m.mode == "cyber" {
							fmt.Println(i.value)
						} else {
							if i.typ == category {
								fmt.Println("category:" + i.value)
							} else if i.typ == app {
								fmt.Println("app:" + i.value)
							}
						}
					}
				}
				return m, tea.Quit
			}
	}
	var cmd tea.Cmd
	m.list, cmd = m.list.Update(msg)
	return m, cmd
}

func (m model) View() string {
	if !m.ready {
		return "Initializing..."
	}
	return m.list.View()
}

func main() {
	var mode string
	flag.StringVar(&mode, "mode", "unpack", "Mode: unpack or cyber")
	flag.Parse()
	var items []list.Item
	if mode == "unpack" {
		items = append(items, item{typ: header, title: "Categories"})
		items = append(items, item{typ: category, title: "Add-Ons", desc: "Install all add-ons", value: "add-ons"})
		items = append(items, item{typ: category, title: "Gaming", desc: "Install all gaming tools including Roblox", value: "gaming"})
		items = append(items, item{typ: category, title: "Cybersecurity", desc: "Setup cybersecurity container", value: "cybersecurity"})
		items = append(items, item{typ: category, title: "Devtools", desc: "Install development tools", value: "devtools"})
		items = append(items, item{typ: category, title: "Emulators", desc: "Install emulators", value: "emulators"})
		items = append(items, item{typ: category, title: "Hacker Mode", desc: "Install hacker mode", value: "hacker-mode"})
		items = append(items, item{typ: category, title: "Gaming No Roblox", desc: "Install gaming tools without Roblox", value: "noroblox"})
		items = append(items, item{typ: header, title: "Individual Applications"})
		items = append(items, item{typ: app, title: "wine", desc: "APT - Run Windows apps", value: "wine"})
		items = append(items, item{typ: app, title: "winetricks", desc: "APT - Wine utilities", value: "winetricks"})
		items = append(items, item{typ: app, title: "BoxBuddy", desc: "Flatpak - Manage Flatpaks", value: "io.github.dvlv.boxbuddyrs"})
		items = append(items, item{typ: app, title: "Winezgui", desc: "Flatpak - Wine GUI", value: "it.mijorus.winezgui"})
		items = append(items, item{typ: app, title: "Gearlever", desc: "Flatpak - Utilities", value: "it.mijorus.gearlever"})
		items = append(items, item{typ: app, title: "obs-studio", desc: "APT - Streaming software", value: "obs-studio"})
		items = append(items, item{typ: app, title: "lutris", desc: "APT - Game launcher", value: "lutris"})
		items = append(items, item{typ: app, title: "Steam", desc: "Flatpak - Gaming platform", value: "com.valvesoftware.Steam"})
		items = append(items, item{typ: app, title: "Pika Torrent", desc: "Flatpak - Torrent client", value: "io.github.giantpinkrobots.varia"})
		items = append(items, item{typ: app, title: "ProtonPlus", desc: "Flatpak - Proton manager", value: "com.vysp3r.ProtonPlus"})
		items = append(items, item{typ: app, title: "Heroic Games Launcher", desc: "Flatpak - Epic/GOG launcher", value: "com.heroicgameslauncher.hgl"})
		items = append(items, item{typ: app, title: "protontricks", desc: "Flatpak - Proton utils", value: "protontricks"})
		items = append(items, item{typ: app, title: "Discord", desc: "Flatpak - Chat app", value: "com.discordapp.Discord"})
		items = append(items, item{typ: app, title: "Roblox", desc: "Flatpak - Roblox player", value: "roblox"})
		items = append(items, item{typ: app, title: "Roblox Studio", desc: "Flatpak - Roblox studio", value: "roblox-studio"})
		items = append(items, item{typ: app, title: "Atom", desc: "Flatpak - Text editor", value: "io.atom.Atom"})
		items = append(items, item{typ: app, title: "shadPS4", desc: "Flatpak - PS4 emulator", value: "org.shadps4.shadPS4"})
		items = append(items, item{typ: app, title: "Ryujinx", desc: "Flatpak - Switch emulator", value: "io.github.ryubing.Ryujinx"})
		items = append(items, item{typ: app, title: "DOSBox-X", desc: "Flatpak - DOS emulator", value: "com.dosbox_x.DOSBox-X"})
		items = append(items, item{typ: app, title: "RPCS3", desc: "Snap - PS3 emulator", value: "rpcs3-emu"})
		items = append(items, item{typ: app, title: "gamescope", desc: "APT - Gaming session manager", value: "gamescope"})
	} else if mode == "cyber" {
		items = append(items, item{typ: category, title: "All", desc: "Install all tools", value: "all"})
		items = append(items, item{typ: header, title: "Categories"})
		out, err := exec.Command("distrobox-enter", "-n", "cybersecurity", "--", "bash", "-c", "pacman -Sg | grep '^blackarch-'").Output()
		if err != nil {
			fmt.Printf("Error getting categories: %v\n", err)
			os.Exit(1)
		}
		lines := strings.Split(string(out), "\n")
		for _, line := range lines {
			line = strings.TrimSpace(line)
			if line != "" {
				items = append(items, item{typ: category, title: line, desc: "BlackArch category", value: line})
			}
		}
	} else {
		fmt.Println("Invalid mode")
		os.Exit(1)
	}
	p := tea.NewProgram(newModel(items, mode), tea.WithAltScreen())
	if _, err := p.Run(); err != nil {
		fmt.Printf("Error: %v\n", err)
		os.Exit(1)
	}
}
