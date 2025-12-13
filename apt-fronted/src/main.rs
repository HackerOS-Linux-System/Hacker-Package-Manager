package main

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"strconv"
	"strings"

	"github.com/schollz/progressbar/v3"
)

const (
	Reset     = "\033[0m"
	Bold      = "\033[1m"
	Underline = "\033[4m"
	Black     = "\033[30m"
	Red       = "\033[31m"
	Green     = "\033[32m"
	Yellow    = "\033[33m"
	Blue      = "\033[34m"
	Magenta   = "\033[35m"
	Cyan      = "\033[36m"
	White     = "\033[37m"
)

func Colored(text, color string, bold, underline bool) string {
	c := ""
	switch strings.ToUpper(color) {
		case "BLACK":
			c = Black
		case "RED":
			c = Red
		case "GREEN":
			c = Green
		case "YELLOW":
			c = Yellow
		case "BLUE":
			c = Blue
		case "MAGENTA":
			c = Magenta
		case "CYAN":
			c = Cyan
		case "WHITE":
			c = White
		default:
			c = Reset
	}
	if bold {
		c += Bold
	}
	if underline {
		c += Underline
	}
	return c + text + Reset
}

type Package struct {
	Name    string
	Version string
	Repo    string
	Arch    string
}

type ParsedOutput struct {
	Installing    []Package
	Upgrading     []Package
	Removing      []Package
	DownloadSize  string
	InstalledSize string
	Summary       [3]int // 0: install, 1: upgrade, 2: remove
}

func ParseAptSimulate(output string) ParsedOutput {
	installing := []Package{}
	upgrading := []Package{}
	removing := []Package{}
	downloadSize := "0"
	installedSize := "0"
	summary := [3]int{0, 0, 0}

	lines := strings.Split(output, "\n")
	instRe := regexp.MustCompile(`Inst (\S+) (?:\[(\S+)\] )?\((\S+) ([\S/]+) (?:\[(\S+)\])?\)`)
	remvRe := regexp.MustCompile(`Remv (\S+) \[(\S+)\]`)
	downloadRe := regexp.MustCompile(`Need to get ([\d.,]+ [kMG]?B) of archives.`)
	installedRe := regexp.MustCompile(`After this operation, ([\d.,]+ [kMG]?B) (?:of additional disk space will be used|disk space will be freed).`)
	summaryRe := regexp.MustCompile(`(\d+) (?:packages? )?upgraded, (\d+) newly installed, (\d+) to remove and (\d+) not upgraded.`)

	for _, line := range lines {
		if instMatch := instRe.FindStringSubmatch(line); instMatch != nil {
			name := instMatch[1]
			currentVer := instMatch[2]
			newVer := instMatch[3]
			repo := instMatch[4]
			arch := instMatch[5]
			if arch == "" {
				arch = "unknown"
			}
			version := newVer
			if version == "" {
				version = currentVer
			}
			pkg := Package{Name: name, Version: version, Repo: repo, Arch: arch}
			if currentVer != "" {
				upgrading = append(upgrading, pkg)
			} else {
				installing = append(installing, pkg)
			}
		} else if remvMatch := remvRe.FindStringSubmatch(line); remvMatch != nil {
			name := remvMatch[1]
			ver := remvMatch[2]
			removing = append(removing, Package{Name: name, Version: ver, Repo: "N/A", Arch: "unknown"})
		} else if downloadMatch := downloadRe.FindStringSubmatch(line); downloadMatch != nil {
			downloadSize = downloadMatch[1]
		} else if installedMatch := installedRe.FindStringSubmatch(line); installedMatch != nil {
			installedSize = installedMatch[1]
		} else if summaryMatch := summaryRe.FindStringSubmatch(line); summaryMatch != nil {
			upgrade, _ := strconv.Atoi(summaryMatch[1])
			install, _ := strconv.Atoi(summaryMatch[2])
			remove, _ := strconv.Atoi(summaryMatch[3])
			summary = [3]int{install, upgrade, remove}
		}
	}
	return ParsedOutput{
		Installing:    installing,
		Upgrading:     upgrading,
		Removing:      removing,
		DownloadSize:  downloadSize,
		InstalledSize: installedSize,
		Summary:       summary,
	}
}

func RunCommand(cmdArgs []string, simulate, stream bool) (string, error) {
	if simulate {
		cmdArgs = append(cmdArgs, "-qq") // quieter for simulation
	}
	cmd := exec.Command(cmdArgs[0], cmdArgs[1:]...)
	if !stream {
		output, err := cmd.CombinedOutput()
		if err != nil {
			fmt.Println(Colored(fmt.Sprintf("Error executing %s", strings.Join(cmdArgs, " ")), "red", false, false))
			return "", err
		}
		return string(output), nil
	}
	var output strings.Builder
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return "", err
	}
	if err := cmd.Start(); err != nil {
		return "", err
	}
	scanner := bufio.NewScanner(stdout)
	for scanner.Scan() {
		line := scanner.Text()
		fmt.Println(line)
		output.WriteString(line + "\n")
	}
	if err := cmd.Wait(); err != nil {
		fmt.Println(Colored(fmt.Sprintf("Error executing %s", strings.Join(cmdArgs, " ")), "red", false, false))
		return output.String(), err
	}
	return output.String(), nil
}

func DisplayDnfStyle(parsed ParsedOutput, action string) {
	fmt.Println(Colored("\nDependencies resolved.", "cyan", true, false))
	fmt.Println(Colored("==========================================================================================", "white", false, false))
	fmt.Printf(" %-35s %-12s %-25s %-20s\n", Colored("Package", "yellow", true, false), Colored("Arch", "yellow", true, false), Colored("Version", "yellow", true, false), Colored("Repository", "yellow", true, false))
	fmt.Println(Colored("==========================================================================================", "white", false, false))

	if len(parsed.Installing) > 0 {
		fmt.Println(Colored("Installing:", "green", true, false))
		for _, pkg := range parsed.Installing {
			fmt.Printf(" %-35s %-12s %-25s %-20s\n", Colored(pkg.Name, "green", false, false), pkg.Arch, pkg.Version, pkg.Repo)
		}
	}
	if len(parsed.Upgrading) > 0 {
		fmt.Println(Colored("Upgrading:", "blue", true, false))
		for _, pkg := range parsed.Upgrading {
			fmt.Printf(" %-35s %-12s %-25s %-20s\n", Colored(pkg.Name, "blue", false, false), pkg.Arch, pkg.Version, pkg.Repo)
		}
	}
	if len(parsed.Removing) > 0 {
		fmt.Println(Colored("Removing:", "red", true, false))
		for _, pkg := range parsed.Removing {
			fmt.Printf(" %-35s %-12s %-25s %-20s\n", Colored(pkg.Name, "red", false, false), pkg.Arch, pkg.Version, pkg.Repo)
		}
	}

	fmt.Println("\n" + Colored("Transaction Summary", "cyan", true, false))
	fmt.Println(Colored("==========================================================================================", "white", false, false))
	fmt.Printf("%s %d Packages\n", Colored("Install", "green", false, false), parsed.Summary[0])
	fmt.Printf("%s %d Packages\n", Colored("Upgrade", "blue", false, false), parsed.Summary[1])
	fmt.Printf("%s %d Packages\n", Colored("Remove", "red", false, false), parsed.Summary[2])
	fmt.Printf("\n%s %s\n", Colored("Total download size:", "magenta", false, false), parsed.DownloadSize)
	if action == "install" || action == "upgrade" {
		fmt.Printf("%s %s\n", Colored("Installed size:", "magenta", false, false), parsed.InstalledSize)
	} else if action == "remove" {
		fmt.Printf("%s %s\n", Colored("Freed size:", "magenta", false, false), parsed.InstalledSize)
	}
	fmt.Println(Colored("==========================================================================================", "white", false, false) + "\n")
}

func ColorOutput(line string) string {
	line = strings.TrimSpace(line)
	if strings.Contains(line, "Setting up") || strings.Contains(line, "Installing") || strings.Contains(line, "Unpacking") {
		return Colored(line, "green", false, false) + "\n"
	} else if strings.Contains(line, "Removing") {
		return Colored(line, "red", false, false) + "\n"
	} else if strings.Contains(line, "Downloading") || strings.Contains(line, "Get:") {
		return Colored(line, "yellow", false, false) + "\n"
	} else if strings.Contains(line, "Reading") || strings.Contains(line, "Building") {
		return Colored(line, "cyan", false, false) + "\n"
	} else if strings.Contains(line, "Hit:") || strings.Contains(line, "Ign:") {
		return Colored(line, "white", false, false) + "\n"
	}
	return line + "\n"
}

func ConfirmAction() bool {
	for {
		fmt.Print(Colored("Do you want to continue? [Y/n] ", "yellow", false, false))
		var response string
		_, err := fmt.Scanln(&response)
		if err != nil {
			response = ""
		}
		response = strings.ToLower(strings.TrimSpace(response))
		if response == "" || response == "y" || response == "yes" {
			return true
		} else if response == "n" || response == "no" {
			return false
		} else {
			fmt.Println(Colored("Please enter Y or N.", "red", false, false))
		}
	}
}

func RunCommandWithProgress(cmdArgs []string) (string, error) {
	cmd := exec.Command(cmdArgs[0], cmdArgs[1:]...)
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return "", err
	}
	if err := cmd.Start(); err != nil {
		return "", err
	}

	bar := progressbar.NewOptions(100,
				      progressbar.OptionSetDescription(Colored("Progress", "blue", false, false)),
				      progressbar.OptionSetTheme(progressbar.Theme{
					      Saucer:        "=",
					      SaucerHead:    ">",
					      SaucerPadding: " ",
					      BarStart:      "[",
					      BarEnd:        "]",
				      }),
			       progressbar.OptionShowBytes(false),
				      progressbar.OptionShowCount(),
				      progressbar.OptionSetWidth(20),
				      progressbar.OptionSetPredictTime(true),
				      progressbar.OptionSetElapsedTime(true),
	)

	var output strings.Builder
	scanner := bufio.NewScanner(stdout)
	for scanner.Scan() {
		line := scanner.Text()
		coloredLine := ColorOutput(line)
		fmt.Print(coloredLine)
		output.WriteString(line + "\n")
		if strings.Contains(line, "%") {
			parts := strings.Split(line, "%")
			if len(parts) > 0 {
				last := strings.TrimSpace(parts[0])
				words := strings.Split(last, " ")
				if len(words) > 0 {
					pstr := words[len(words)-1]
					percent, err := strconv.Atoi(pstr)
					if err == nil && percent >= 0 && percent <= 100 {
						bar.Set(percent)
					}
				}
			}
		}
	}
	bar.Finish()
	fmt.Println()

	if err := cmd.Wait(); err != nil {
		fmt.Println(Colored("Error executing command.", "red", false, false))
		return output.String(), err
	}
	return output.String(), nil
}

func HandleInstall(packages []string) {
	if len(packages) == 0 {
		fmt.Println(Colored("No packages specified for install.", "red", false, false))
		return
	}
	cmd := append([]string{"sudo", "apt", "install", "-y"}, packages...)
	simCmd := append([]string{"sudo", "apt", "install"}, packages...)
	simCmd = append(simCmd, "-s")
	simOutput, err := RunCommand(simCmd, true, false)
	if err != nil {
		return
	}
	parsed := ParseAptSimulate(simOutput)
	DisplayDnfStyle(parsed, "install")
	if ConfirmAction() {
		fmt.Println(Colored("Running transaction", "cyan", false, false))
		_, _ = RunCommandWithProgress(cmd)
	} else {
		fmt.Println(Colored("Transaction cancelled.", "yellow", false, false))
	}
}

func HandleRemove(packages []string) {
	if len(packages) == 0 {
		fmt.Println(Colored("No packages specified for remove.", "red", false, false))
		return
	}
	cmd := append([]string{"sudo", "apt", "remove", "-y"}, packages...)
	simCmd := append([]string{"sudo", "apt", "remove"}, packages...)
	simCmd = append(simCmd, "-s")
	simOutput, err := RunCommand(simCmd, true, false)
	if err != nil {
		return
	}
	parsed := ParseAptSimulate(simOutput)
	DisplayDnfStyle(parsed, "remove")
	if ConfirmAction() {
		fmt.Println(Colored("Running transaction", "cyan", false, false))
		_, _ = RunCommandWithProgress(cmd)
	} else {
		fmt.Println(Colored("Transaction cancelled.", "yellow", false, false))
	}
}

func HandleUpdate() {
	fmt.Println(Colored("Updating package lists...", "cyan", false, false))
	updateCmd := []string{"sudo", "apt", "update"}
	_, _ = RunCommandWithProgress(updateCmd)

	upgradeCmd := []string{"sudo", "apt", "upgrade", "-y"}
	simCmd := []string{"sudo", "apt", "upgrade", "-s"}
	simOutput, err := RunCommand(simCmd, true, false)
	if err != nil {
		return
	}
	parsed := ParseAptSimulate(simOutput)
	DisplayDnfStyle(parsed, "upgrade")
	if ConfirmAction() {
		fmt.Println(Colored("Running upgrade", "cyan", false, false))
		_, _ = RunCommandWithProgress(upgradeCmd)
	} else {
		fmt.Println(Colored("Upgrade cancelled.", "yellow", false, false))
	}
}

func HandleClean() {
	autocleanCmd := []string{"sudo", "apt", "autoclean"}
	autoremoveCmd := []string{"sudo", "apt", "autoremove", "-y"}
	simCmd := []string{"sudo", "apt", "autoremove", "-s"}
	simOutput, err := RunCommand(simCmd, true, false)
	if err != nil {
		return
	}
	parsed := ParseAptSimulate(simOutput)
	DisplayDnfStyle(parsed, "clean")
	if ConfirmAction() {
		fmt.Println(Colored("Running autoclean", "cyan", false, false))
		_, _ = RunCommandWithProgress(autocleanCmd)
		fmt.Println(Colored("Running autoremove", "cyan", false, false))
		_, _ = RunCommandWithProgress(autoremoveCmd)
	} else {
		fmt.Println(Colored("Clean cancelled.", "yellow", false, false))
	}
}

func PrintHelp() {
	fmt.Println(Colored("Enhanced APT Frontend in DNF Style with Colors and Progress", "magenta", true, false))
	fmt.Println("Usage: apt-frontend <command> [options]")
	fmt.Println("Commands:")
	fmt.Println("  install <packages...>   Install packages")
	fmt.Println("  remove <packages...>    Remove packages")
	fmt.Println("  update                  Update and upgrade packages")
	fmt.Println("  clean                   Clean up packages")
}

func main() {
	if len(os.Args) < 2 {
		PrintHelp()
		os.Exit(1)
	}

	command := os.Args[1]
	switch command {
		case "install":
			HandleInstall(os.Args[2:])
		case "remove":
			HandleRemove(os.Args[2:])
		case "update":
			HandleUpdate()
		case "clean":
			HandleClean()
		default:
			PrintHelp()
			os.Exit(1)
	}
}
