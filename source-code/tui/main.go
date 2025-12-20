package main

import (
	"flag"
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"

	"github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/bubbles/list"
	"github.com/charmbracelet/bubbles/textinput"
	"github.com/charmbracelet/lipgloss"
)

type Source string

const (
	Apt     Source = "APT"
	Snap    Source = "SNAP"
	Flatpak Source = "FLATPAK"
	All     Source = "ALL"
)

type Package struct {
	name   string
	source Source
	desc   string
}

func (p Package) Title() string {
	return fmt.Sprintf("%s (%s) - %s", p.name, p.source, p.desc)
}

func (p Package) Description() string {
	return ""
}

func (p Package) FilterValue() string {
	return p.name
}

type InputMode int

const (
	Normal InputMode = iota
	Editing
)

type tickMsg struct{}

type searchResult struct {
	pkgs []Package
	err  error
}

type installResult struct {
	output string
	err    error
}

type removeResult struct {
	output string
	err    error
}

type model struct {
	textInput      textinput.Model
	pkgList        list.Model
	mode           InputMode
	selectedSource Source
	message        string
	dotCount       int
	isSearching    bool
	isInstalling   bool
	isRemoving     bool
	packages       []Package
	quitting       bool
}

var (
	inputStyle   = lipgloss.NewStyle().Border(lipgloss.NormalBorder()).Padding(0, 1)
	listStyle    = lipgloss.NewStyle().Margin(1, 0)
	helpStyle    = lipgloss.NewStyle().Foreground(lipgloss.Color("241"))
	messageStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("11"))
)

func New(initialQuery string) model {
	ti := textinput.New()
	ti.Placeholder = "Search query"
	ti.Width = 50

	l := list.New([]list.Item{}, list.NewDefaultDelegate(), 0, 0)
	l.SetShowHelp(false)
	l.SetShowFilter(false)
	l.SetShowPagination(false)
	l.SetShowTitle(true)

	m := model{
		textInput:      ti,
		pkgList:        l,
		mode:           Normal,
		selectedSource: All,
		message:        "",
		packages:       []Package{},
	}

	if initialQuery != "" {
		m.textInput.SetValue(initialQuery)
		pkgs, _ := searchPackages(initialQuery)
		m.packages = pkgs
		m.updateList()
	}

	return m
}

func (m model) Init() tea.Cmd {
	return textinput.Blink
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd
	var cmds []tea.Cmd

	switch msg := msg.(type) {
		case tea.WindowSizeMsg:
			m.textInput.Width = msg.Width - 4
			m.pkgList.SetWidth(msg.Width)
			m.pkgList.SetHeight(msg.Height - 8) // Approximate for input and help
			return m, nil

		case tea.KeyMsg:
			if m.isSearching || m.isInstalling || m.isRemoving {
				if msg.String() == "q" || msg.String() == "ctrl+c" {
					m.quitting = true
					return m, tea.Quit
				}
				return m, nil
			}

			switch m.mode {
				case Editing:
					switch msg.String() {
						case "esc":
							m.mode = Normal
							m.textInput.Blur()
							return m, nil
						case "enter":
							m.mode = Normal
							m.textInput.Blur()
							return m, nil
					}
					m.textInput, cmd = m.textInput.Update(msg)
					cmds = append(cmds, cmd)
					return m, tea.Batch(cmds...)

						case Normal:
							switch msg.String() {
								case "q", "ctrl+c":
									m.quitting = true
									return m, tea.Quit
								case "e":
									m.mode = Editing
									m.textInput.Focus()
									return m, textinput.Blink
								case "enter":
									query := m.textInput.Value()
									if query == "" {
										m.message = "Enter a search query."
										return m, nil
									}
									m.message = "Searching"
									m.dotCount = 0
									m.isSearching = true
									return m, tea.Batch(searchCmd(query), tick())
								case "i":
									if item, ok := m.pkgList.SelectedItem().(Package); ok {
										m.message = "Installing..."
										m.isInstalling = true
										return m, installCmd(item)
									}
								case "r":
									if item, ok := m.pkgList.SelectedItem().(Package); ok {
										m.message = "Removing..."
										m.isRemoving = true
										return m, removeCmd(item)
									}
								case "a":
									m.selectedSource = Apt
									m.updateList()
									return m, nil
								case "s":
									m.selectedSource = Snap
									m.updateList()
									return m, nil
								case "f":
									m.selectedSource = Flatpak
									m.updateList()
									return m, nil
								case "l":
									m.selectedSource = All
									m.updateList()
									return m, nil
								default:
									m.pkgList, cmd = m.pkgList.Update(msg)
									cmds = append(cmds, cmd)
									return m, tea.Batch(cmds...)
							}
			}

								case tickMsg:
									if m.isSearching {
										m.dotCount = (m.dotCount + 1) % 4
										m.message = "Searching" + strings.Repeat(".", m.dotCount)
										return m, tick()
									}
									return m, nil

								case searchResult:
									m.isSearching = false
									if msg.err != nil {
										m.message = fmt.Sprintf("Search failed: %v", msg.err)
									} else {
										m.packages = msg.pkgs
										if len(m.packages) == 0 {
											m.message = "No packages found."
										} else {
											m.message = ""
										}
										m.updateList()
									}
									return m, nil

								case installResult:
									m.isInstalling = false
									if msg.err != nil {
										m.message = fmt.Sprintf("Install failed: %v", msg.err)
									} else {
										m.message = msg.output
									}
									return m, nil

								case removeResult:
									m.isRemoving = false
									if msg.err != nil {
										m.message = fmt.Sprintf("Remove failed: %v", msg.err)
									} else {
										m.message = msg.output
									}
									return m, nil
	}

	return m, tea.Batch(cmds...)
}

func (m *model) updateList() {
	var items []list.Item
	for _, p := range m.packages {
		if m.selectedSource == All || p.source == m.selectedSource {
			items = append(items, p)
		}
	}
	m.pkgList.Title = fmt.Sprintf("Packages [%s] (a/s/f/l to switch, i:install, r:remove)", m.selectedSource)
	m.pkgList.SetItems(items)
	if len(items) > 0 {
		m.pkgList.Select(0)
	}
}

func (m model) View() string {
	if m.quitting {
		return "Goodbye!\n"
	}

	input := inputStyle.Render(m.textInput.View())
	pkgList := listStyle.Render(m.pkgList.View())

	var help string
	switch m.mode {
		case Normal:
			help = "Press q to exit, e to edit query, Enter to search, a/s/f/l to switch source (APT/SNAP/FLATPAK/ALL), i to install, r to remove, j/k or arrows to navigate."
		case Editing:
			help = "Press Esc to cancel, Enter to confirm editing."
	}

	if m.message != "" {
		help += "\n" + messageStyle.Render(m.message)
	}

	help = helpStyle.Render(help)

	return lipgloss.JoinVertical(lipgloss.Left, input, pkgList, help)
}

func tick() tea.Cmd {
	return tea.Tick(500*time.Millisecond, func(_ time.Time) tea.Msg {
		return tickMsg{}
	})
}

func searchCmd(query string) tea.Cmd {
	return func() tea.Msg {
		pkgs, err := searchPackages(query)
		return searchResult{pkgs: pkgs, err: err}
	}
}

func searchPackages(query string) ([]Package, error) {
	var pkgs []Package

	// Search APT
	cmd := exec.Command("apt-cache", "search", "--names-only", query)
	out, _ := cmd.Output() // Ignore error, proceed if possible
	if len(out) > 0 {
		lines := strings.Split(string(out), "\n")
		for _, line := range lines {
			line = strings.TrimSpace(line)
			if line == "" {
				continue
			}
			parts := strings.SplitN(line, " - ", 2)
			if len(parts) == 2 {
				pkgs = append(pkgs, Package{
					name:   strings.TrimSpace(parts[0]),
					      source: Apt,
					      desc:   strings.TrimSpace(parts[1]),
				})
			}
		}
	}

	// Search Snap
	cmd = exec.Command("snap", "find", query)
	out, _ = cmd.Output()
	if len(out) > 0 {
		lines := strings.Split(string(out), "\n")
		start := 0
		if len(lines) > 0 && strings.Contains(lines[0], "Name") {
			start = 1
		}
		for _, line := range lines[start:] {
			line = strings.TrimSpace(line)
			if line == "" {
				continue
			}
			fields := strings.Fields(line)
			if len(fields) >= 5 {
				name := fields[0]
				desc := strings.Join(fields[4:], " ")
				pkgs = append(pkgs, Package{name: name, source: Snap, desc: desc})
			}
		}
	}

	// Search Flatpak
	cmd = exec.Command("flatpak", "search", query)
	out, _ = cmd.Output()
	if len(out) > 0 {
		lines := strings.Split(string(out), "\n")
		start := 0
		if len(lines) > 0 && strings.Contains(lines[0], "Name") {
			start = 1
		}
		for _, line := range lines[start:] {
			line = strings.TrimSpace(line)
			if line == "" {
				continue
			}
			parts := strings.Split(line, "\t")
			if len(parts) >= 3 {
				name := parts[2]
				desc := fmt.Sprintf("%s - %s", parts[0], parts[1])
				pkgs = append(pkgs, Package{name: name, source: Flatpak, desc: desc})
			}
		}
	}

	return pkgs, nil
}

func installCmd(pkg Package) tea.Cmd {
	return func() tea.Msg {
		output, err := installPackage(pkg)
		return installResult{output: output, err: err}
	}
}

func installPackage(p Package) (string, error) {
	var cmd *exec.Cmd
	switch p.source {
		case Apt:
			cmd = exec.Command("sudo", "apt", "install", "-y", p.name)
		case Snap:
			cmd = exec.Command("sudo", "snap", "install", p.name)
		case Flatpak:
			cmd = exec.Command("sudo", "flatpak", "install", "--assumeyes", p.name)
		default:
			return "Invalid source", nil
	}
	out, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("failed to install %s from %s: %v\n%s", p.name, p.source, err, string(out))
	}
	return fmt.Sprintf("Installed %s from %s", p.name, p.source), nil
}

func removeCmd(pkg Package) tea.Cmd {
	return func() tea.Msg {
		output, err := removePackage(pkg)
		return removeResult{output: output, err: err}
	}
}

func removePackage(p Package) (string, error) {
	var cmd *exec.Cmd
	switch p.source {
		case Apt:
			cmd = exec.Command("sudo", "apt", "remove", "-y", p.name)
		case Snap:
			cmd = exec.Command("sudo", "snap", "remove", p.name)
		case Flatpak:
			cmd = exec.Command("sudo", "flatpak", "uninstall", "--assumeyes", p.name)
		default:
			return "Invalid source", nil
	}
	out, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("failed to remove %s from %s: %v\n%s", p.name, p.source, err, string(out))
	}
	return fmt.Sprintf("Removed %s from %s", p.name, p.source), nil
}

func main() {
	query := flag.String("query", "", "Initial search query")
	flag.Parse()

	p := tea.NewProgram(New(*query), tea.WithAltScreen())
	if _, err := p.Run(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
