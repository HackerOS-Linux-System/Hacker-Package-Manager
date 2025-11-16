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

type FAQ struct {
	Question string
	Answer   string
}

var faqs = []FAQ{
	{
		Question: "Jak instalować pakiety na HackerOS?",
		Answer:   "Użyj 'hacker install {nazwa pakietu}' lub 'sudo apt install {nazwa pakietu}' lub możesz użyć aplikacji Software (GNOME Software).",
	},
	{
		Question: "Jak usuwać pakiety?",
		Answer:   "Użyj 'hacker remove {nazwa pakietu}' lub 'sudo apt remove {nazwa pakietu}' lub możesz użyć aplikacji Software (GNOME Software).",
	},
	{
		Question: "Jak zaktualizować system?",
		Answer:   "Polecane dla HackerOS: użyj aplikacji 'Update System' lub komendy 'hacker update'.",
	},
	{
		Question: "Skąd mogę zdobyć oprogramowanie?",
		Answer:   "Instaluj za pomocą 'hacker unpack {nazwa zestawu}' lub 'hacker unpack select' aby wybrać konkretne aplikacje. Możesz również instalować za pomocą apt, snap lub flatpak.",
	},
	{
		Question: "Co to jest HackerOS?",
		Answer:   "HackerOS to dystrybucja Linux oparta na debianie testowym, skupiona na gamingu, cybersecurity i narzędziach dla hackerów.",
	},
	{
		Question: "Jak zmienić hasło?",
		Answer:   "Użyj komendy 'passwd' w terminalu.",
	},
	{
		Question: "Jak zrestartować system?",
		Answer:   "Użyj 'sudo reboot' lub wybierz opcję restartu z menu.",
	},
	{
		Question: "Jak zainstalować sterowniki GPU dla NVIDIA?",
		Answer:   "Użyj 'sudo apt install nvidia-driver' i zrestartuj system.",
	},
	{
		Question: "Co to jest distrobox?",
		Answer:   "Distrobox to narzędzie do tworzenia i zarządzania kontenerami z innymi dystrybucjami Linuxa w twoim systemie.",
	},
	{
		Question: "Jak uruchomić aplikację Windows na HackerOS?",
		Answer:   "Zainstaluj Wine za pomocą 'hacker unpack add-ons' i użyj 'wine {plik.exe}'.",
	},
	{
		Question: "Jak skonfigurować cybersecurity tools?",
		Answer:   "Użyj 'hacker unpack cybersecurity' aby ustawić kontener z BlackArch tools.",
	},
	{
		Question: "Jak grać w gry na HackerOS?",
		Answer:   "Zainstaluj gaming tools za pomocą 'hacker unpack gaming' i użyj Steam, Lutris lub Heroic Games Launcher.",
	},
	{
		Question: "Jak sprawdzić logi systemowe?",
		Answer:   "Użyj 'hacker system logs' lub 'journalctl -xe'.",
	},
	{
		Question: "Jak wejść do kontenera distrobox?",
		Answer:   "Użyj 'hacker enter {nazwa kontenera}'.",
	},
}

type item struct {
	faq FAQ
}

func (i item) Title() string       { return i.faq.Question }
func (i item) Description() string { return "" }
func (i item) FilterValue() string { return i.faq.Question }

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
	items = append(items, item{FAQ{Question: "Exit", Answer: "Press to exit"}})
	for _, f := range faqs {
		items = append(items, item{f})
	}
	delegate := list.NewDefaultDelegate()
	delegate.Styles.SelectedTitle = delegate.Styles.SelectedTitle.Foreground(lipgloss.Color("#FF75CB")).Bold(true)
	l := list.New(items, delegate, 0, 0)
	l.Title = "HackerOS Documentation & FAQ"
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
			m.viewport.Width = msg.Width / 2 - 4
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
						m.viewport.SetContent(faqs[m.selected].Answer)
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
	rightContent := "Select a question from the list to view the answer.\n\nPress 'enter' to select, 'esc' to go back, 'q' to quit."
	if m.mode == detailsMode {
		rightContent = m.viewport.View()
	}
	right := lipgloss.NewStyle().
	Width(m.viewport.Width + 4).
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
