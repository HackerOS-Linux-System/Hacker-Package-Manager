package main

import (
	"bufio"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/progress"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/pterm/pterm"
)

const (
	repoURL     = "https://raw.githubusercontent.com/HackerOS-Linux-System/Hacker-Package-Manager/main/community/repo/repo.hacker"
	repoFile    = "/tmp/repo.hacker"
	tmpDirBase  = "/tmp/community-packages"
	templateURL = "https://github.com/Bytes-Repository/hpm-example-repo"
)

var repoMap map[string]string

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(1)
	}

	cmd := os.Args[1]
	args := os.Args[2:]

	// Load repo map
	err := loadRepoMap()
	if err != nil {
		pterm.Error.Println("Failed to load repo map:", err)
		os.Exit(1)
	}

	switch cmd {
	case "install":
		if len(args) != 1 {
			pterm.Error.Println("Usage: community install {package}")
			os.Exit(1)
		}
		err := handleInstall(args[0])
		if err != nil {
			pterm.Error.Println("Install failed:", err)
		} else {
			pterm.Success.Println("Install completed successfully.")
		}
	case "remove":
		if len(args) != 1 {
			pterm.Error.Println("Usage: community remove {package}")
			os.Exit(1)
		}
		err := handleRemove(args[0])
		if err != nil {
			pterm.Error.Println("Remove failed:", err)
		} else {
			pterm.Success.Println("Remove completed successfully.")
		}
	case "clean":
		err := handleClean()
		if err != nil {
			pterm.Error.Println("Clean failed:", err)
		} else {
			pterm.Success.Println("Clean completed successfully.")
		}
	case "template":
		handleTemplate()
	case "update":
		pterm.Info.Println("Update feature not implemented yet.")
	default:
		printUsage()
		os.Exit(1)
	}
}

func printUsage() {
	pterm.DefaultHeader.WithFullWidth().Println("Community Package Manager")
	pterm.Println("Commands:")
	pterm.Println("  install {package} - Install a package")
	pterm.Println("  remove {package}  - Remove a package")
	pterm.Println("  clean             - Clean temporary files")
	pterm.Println("  template          - Show template repository link")
	pterm.Println("  update            - Update (not implemented)")
}

func loadRepoMap() error {
	// Check if repo file exists, otherwise download
	if _, err := os.Stat(repoFile); os.IsNotExist(err) {
		err := downloadRepoFile()
		if err != nil {
			return err
		}
	}

	file, err := os.Open(repoFile)
	if err != nil {
		return err
	}
	defer file.Close()

	repoMap = make(map[string]string)
	scanner := bufio.NewScanner(file)
	inArray := false

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "[" {
			inArray = true
			continue
		}
		if line == "]" {
			inArray = false
			continue
		}
		if inArray && strings.Contains(line, "->") {
			parts := strings.SplitN(line, "->", 2)
			if len(parts) == 2 {
				key := strings.TrimSpace(parts[0])
				value := strings.TrimSpace(parts[1])
				// Remove trailing comma if present
				if strings.HasSuffix(value, ",") {
					value = strings.TrimSuffix(value, ",")
				}
				repoMap[key] = value
			}
		}
	}

	if err := scanner.Err(); err != nil {
		return err
	}

	return nil
}

func downloadRepoFile() error {
	pterm.Info.Println("Downloading repo.hacker...")
	resp, err := http.Get(repoURL)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("failed to download: %s", resp.Status)
	}

	file, err := os.Create(repoFile)
	if err != nil {
		return err
	}
	defer file.Close()

	_, err = io.Copy(file, resp.Body)
	return err
}

func handleInstall(pkg string) error {
	url, ok := repoMap[pkg]
	if !ok {
		return fmt.Errorf("package %s not found in repo", pkg)
	}

	tmpDir := filepath.Join(tmpDirBase, pkg)
	err := os.MkdirAll(tmpDir, 0755)
	if err != nil {
		return err
	}

	pterm.Info.Println("Cloning repository:", url)
	err = gitClone(url, tmpDir)
	if err != nil {
		return err
	}

	script := filepath.Join(tmpDir, "install.hacker")
	return runScriptWithProgress(script)
}

func handleRemove(pkg string) error {
	url, ok := repoMap[pkg]
	if !ok {
		return fmt.Errorf("package %s not found in repo", pkg)
	}

	tmpDir := filepath.Join(tmpDirBase, pkg)
	err := os.MkdirAll(tmpDir, 0755)
	if err != nil {
		return err
	}

	pterm.Info.Println("Cloning repository:", url)
	err = gitClone(url, tmpDir)
	if err != nil {
		return err
	}

	script := filepath.Join(tmpDir, "remove.hacker")
	return runScriptWithProgress(script)
}

func gitClone(url, dir string) error {
	cmd := exec.Command("git", "clone", url, dir)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func runScriptWithProgress(script string) error {
	if _, err := os.Stat(script); os.IsNotExist(err) {
		return fmt.Errorf("script %s not found", script)
	}

	pterm.Info.Println("Running script:", script)

	// Simulate progress for script execution (since we can't hook into bash progress easily)
	p := tea.NewProgram(initialModel())
	go func() {
		// Simulate 10 seconds of work
		time.Sleep(10 * time.Second)
		p.Quit()
	}()

	if _, err := p.Run(); err != nil {
		return err
	}

	// Actually run the script
	cmd := exec.Command("bash", script)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func handleClean() error {
	pterm.Info.Println("Cleaning temporary files...")
	return os.RemoveAll(tmpDirBase)
}

func handleTemplate() {
	pterm.Info.Println("Template repository:", templateURL)
}

// Bubble Tea Progress Model
type model struct {
	progress progress.Model
}

func initialModel() model {
	return model{
		progress: progress.New(progress.WithDefaultGradient()),
	}
}

func (m model) Init() tea.Cmd {
	return tickCmd()
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		return m, tea.Quit

	case tea.WindowSizeMsg:
		m.progress.Width = msg.Width - 4
		if m.progress.Width > 60 {
			m.progress.Width = 60
		}
		return m, nil

	case tickMsg:
		if m.progress.Percent() >= 1.0 {
			return m, tea.Quit
		}
		cmd := m.progress.IncrPercent(0.25)
		return m, tea.Batch(tickCmd(), cmd)
	}

	return m, nil
}

func (m model) View() string {
	return "\n" +
		m.progress.View() + "\n\n" +
		"Running script... Press any key to quit\n"
}

type tickMsg time.Time

func tickCmd() tea.Cmd {
	return tea.Tick(time.Second*1, func(t time.Time) tea.Msg {
		return tickMsg(t)
	})
}
