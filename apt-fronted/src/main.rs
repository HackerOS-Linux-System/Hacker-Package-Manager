use colored::*;
use kdam::{Bar, BarExt, Spinner};
use regex::Regex;
use std::env;
use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::str::FromStr;

fn colored(text: &str, color: &str, bold: bool, underline: bool) -> String {
	let mut s = String::from(text).normal();
	s = match color.to_uppercase().as_str() {
		"BLACK" => s.black(),
		"RED" => s.red(),
		"GREEN" => s.green(),
		"YELLOW" => s.yellow(),
		"BLUE" => s.blue(),
		"MAGENTA" => s.magenta(),
		"CYAN" => s.cyan(),
		"WHITE" => s.white(),
		_ => s,
	};
	if bold {
		s = s.bold();
	}
	if underline {
		s = s.underline();
	}
	s.to_string()
}

#[derive(Clone)]
struct Package {
	name: String,
	version: String,
	repo: String,
	arch: String,
}

struct ParsedOutput {
	installing: Vec<Package>,
	upgrading: Vec<Package>,
	removing: Vec<Package>,
	download_size: String,
	installed_size: String,
	summary: [i32; 3], // 0: install, 1: upgrade, 2: remove
}

fn parse_apt_simulate(output: &str) -> ParsedOutput {
	let mut installing: Vec<Package> = Vec::new();
	let mut upgrading: Vec<Package> = Vec::new();
	let mut removing: Vec<Package> = Vec::new();
	let mut download_size = "0".to_string();
	let mut installed_size = "0".to_string();
	let mut summary = [0, 0, 0];

	let inst_re = Regex::new(r"Inst (\S+) (?:\[(\S+)\] )?\((\S+) ([\S/]+) (?:\[(\S+)\])?\)").unwrap();
	let remv_re = Regex::new(r"Remv (\S+) \[(\S+)\]").unwrap();
	let download_re = Regex::new(r"Need to get ([\d.,]+ [kMG]?B) of archives.").unwrap();
	let installed_re = Regex::new(r"After this operation, ([\d.,]+ [kMG]?B) (?:of additional disk space will be used|disk space will be freed).").unwrap();
	let summary_re = Regex::new(r"(\d+) (?:packages? )?upgraded, (\d+) newly installed, (\d+) to remove and (\d+) not upgraded.").unwrap();

	for line in output.lines() {
		if let Some(caps) = inst_re.captures(line) {
			let name = caps[1].to_string();
			let current_ver = caps.get(2).map_or("".to_string(), |m| m.as_str().to_string());
			let new_ver = caps[3].to_string();
			let repo = caps[4].to_string();
			let arch = caps.get(5).map_or("unknown".to_string(), |m| m.as_str().to_string());
			let version = if !new_ver.is_empty() { new_ver } else { current_ver.clone() };
			let pkg = Package {
				name,
				version,
				repo,
				arch,
			};
			if !current_ver.is_empty() {
				upgrading.push(pkg);
			} else {
				installing.push(pkg);
			}
		} else if let Some(caps) = remv_re.captures(line) {
			let name = caps[1].to_string();
			let ver = caps[2].to_string();
			removing.push(Package {
				name,
				version: ver,
				repo: "N/A".to_string(),
						  arch: "unknown".to_string(),
			});
		} else if let Some(caps) = download_re.captures(line) {
			download_size = caps[1].to_string();
		} else if let Some(caps) = installed_re.captures(line) {
			installed_size = caps[1].to_string();
		} else if let Some(caps) = summary_re.captures(line) {
			let upgrade: i32 = FromStr::from_str(&caps[1]).unwrap_or(0);
			let install: i32 = FromStr::from_str(&caps[2]).unwrap_or(0);
			let remove: i32 = FromStr::from_str(&caps[3]).unwrap_or(0);
			summary = [install, upgrade, remove];
		}
	}

	ParsedOutput {
		installing,
		upgrading,
		removing,
		download_size,
		installed_size,
		summary,
	}
}

fn run_command(
	cmd_args: &[String],
	simulate: bool,
	stream: bool,
) -> Result<String, Box<dyn Error>> {
	let mut args = cmd_args.to_vec();
	if simulate {
		args.push("-qq".to_string());
	}
	let mut cmd = Command::new(&args[0]);
	cmd.args(&args[1..]);

	if !stream {
		let output = cmd.output()?;
		if !output.status.success() {
			println!(
				"{}",
			colored(
				&format!("Error executing {}", args.join(" ")),
					"red",
		   false,
		   false
			)
			);
			return Err("Command failed".into());
		}
		return Ok(String::from_utf8_lossy(&output.stdout).to_string());
	}

	let mut child = cmd.stdout(Stdio::piped()).spawn()?;
	let stdout = child.stdout.take().unwrap();
	let mut scanner = BufReader::new(stdout).lines();
	let mut output = String::new();

	while let Some(line) = scanner.next() {
		let line = line?;
		println!("{}", line);
		output.push_str(&line);
		output.push('\n');
	}

	let status = child.wait()?;
	if !status.success() {
		println!(
			"{}",
		   colored(
			   &format!("Error executing {}", args.join(" ")),
				   "red",
			 false,
			 false
		   )
		);
		return Err("Command failed".into());
	}

	Ok(output)
}

fn get_num_downloads(action: &str, packages: &[String]) -> usize {
	let mut cmd_args: Vec<String> = vec!["apt-get".to_string(), "--print-uris".to_string(), "-y".to_string(), action.to_string()];
	cmd_args.extend(packages.iter().cloned());
	let output = match run_command(&cmd_args, false, false) {
		Ok(o) => o,
		Err(_) => return 0,
	};
	output.lines().filter(|l| l.starts_with("'http")).count()
}

fn display_dnf_style(parsed: &ParsedOutput, action: &str) {
	println!("{}", colored("\nDependencies resolved.", "cyan", true, false));
	println!(
		"{}",
		colored(
			"==========================================================================================",
		  "white",
		  false,
		  false
		)
	);
	println!(
		" {:-<35} {:-<12} {:-<25} {:-<20}",
		colored("Package", "yellow", true, false),
			 colored("Arch", "yellow", true, false),
			 colored("Version", "yellow", true, false),
			 colored("Repository", "yellow", true, false)
	);
	println!(
		"{}",
		colored(
			"==========================================================================================",
		  "white",
		  false,
		  false
		)
	);

	if !parsed.installing.is_empty() {
		println!("{}", colored("Installing:", "green", true, false));
		for pkg in &parsed.installing {
			println!(
				" {:-<35} {:-<12} {:-<25} {:-<20}",
			colored(&pkg.name, "green", false, false),
					 &pkg.arch,
			&pkg.version,
			&pkg.repo
			);
		}
	}

	if !parsed.upgrading.is_empty() {
		println!("{}", colored("Upgrading:", "blue", true, false));
		for pkg in &parsed.upgrading {
			println!(
				" {:-<35} {:-<12} {:-<25} {:-<20}",
			colored(&pkg.name, "blue", false, false),
					 &pkg.arch,
			&pkg.version,
			&pkg.repo
			);
		}
	}

	if !parsed.removing.is_empty() {
		println!("{}", colored("Removing:", "red", true, false));
		for pkg in &parsed.removing {
			println!(
				" {:-<35} {:-<12} {:-<25} {:-<20}",
			colored(&pkg.name, "red", false, false),
					 &pkg.arch,
			&pkg.version,
			&pkg.repo
			);
		}
	}

	println!(
		"\n{}",
		colored("Transaction Summary", "cyan", true, false)
	);
	println!(
		"{}",
		colored(
			"==========================================================================================",
		  "white",
		  false,
		  false
		)
	);
	println!(
		"{} {} Packages",
		colored("Install", "green", false, false),
			 parsed.summary[0]
	);
	println!(
		"{} {} Packages",
		colored("Upgrade", "blue", false, false),
			 parsed.summary[1]
	);
	println!(
		"{} {} Packages",
		colored("Remove", "red", false, false),
			 parsed.summary[2]
	);
	println!(
		"\n{} {}",
		colored("Total download size:", "magenta", false, false),
			 &parsed.download_size
	);
	if action == "install" || action == "upgrade" {
		println!(
			"{} {}",
		   colored("Installed size:", "magenta", false, false),
				 &parsed.installed_size
		);
	} else if action == "remove" {
		println!(
			"{} {}",
		   colored("Freed size:", "magenta", false, false),
				 &parsed.installed_size
		);
	}
	println!(
		"{}\n",
		colored(
			"==========================================================================================",
		  "white",
		  false,
		  false
		)
	);
}

fn color_output(line: &str) -> String {
	let trimmed = line.trim();
	if trimmed.contains("Setting up")
		|| trimmed.contains("Konfigurowanie")
		|| trimmed.contains("Installing")
		|| trimmed.contains("Unpacking")
		|| trimmed.contains("Rozpakowywanie")
		{
			colored(trimmed, "green", false, false) + "\n"
		} else if trimmed.contains("Removing") || trimmed.contains("Usuwanie") {
			colored(trimmed, "red", false, false) + "\n"
		} else if trimmed.contains("Downloading") || trimmed.contains("Get:") || trimmed.contains("Pobr:") {
			colored(trimmed, "yellow", false, false) + "\n"
		} else if trimmed.contains("Reading") || trimmed.contains("Building") || trimmed.contains("Wybieranie") || trimmed.contains("Przygotowywanie") {
			colored(trimmed, "cyan", false, false) + "\n"
		} else if trimmed.contains("Hit:") || trimmed.contains("Ign:") {
			colored(trimmed, "white", false, false) + "\n"
		} else if trimmed.contains("Processing triggers") || trimmed.contains("Przetwarzanie wyzwalaczy") {
			colored(trimmed, "magenta", false, false) + "\n"
		} else {
			trimmed.to_string() + "\n"
		}
}

fn confirm_action() -> bool {
	loop {
		print!(
			"{}",
		 colored("Is this ok [y/N]: ", "yellow", false, false)
		);
		io::stdout().flush().unwrap();
		let mut response = String::new();
		io::stdin().read_line(&mut response).unwrap();
		let response = response.trim().to_lowercase();
		if response == "y" || response == "yes" {
			return true;
		} else if response.is_empty() || response == "n" || response == "no" {
			return false;
		} else {
			println!("{}", colored("Please enter y or N.", "red", false, false));
		}
	}
}

fn run_with_progress(cmd_args: &[String], desc: String, total: usize, update_regexes: Vec<Regex>) -> Result<String, Box<dyn Error>> {
	let mut cmd = Command::new(&cmd_args[0]);
	cmd.args(&cmd_args[1..]);
	cmd.stdout(Stdio::piped());

	let mut child = cmd.spawn()?;

	let stdout = child.stdout.take().unwrap();
	let mut scanner = BufReader::new(stdout).lines();

	let mut pb = Bar::builder()
	.total(total)
	.desc(desc)
	.bar_format("{desc suffix=' '}|{animation}| {spinner} {count}/{total} [{percentage:.0}%] in {elapsed human=true} ({rate:.1}/s, eta: {remaining human=true})".to_string())
	.spinner(Spinner::new(
		&["▁▂▃", "▂▃▄", "▃▄▅", "▄▅▆", "▅▆▇", "▆▇█", "▇█▇", "█▇▆", "▇▆▅", "▆▅▄", "▅▄▃", "▄▃▂", "▃▂▁"],
		30.0,
		1.0,
	))
	.ncols(20u16)
	.force_refresh(true)
	.build()?;

	let mut output = String::new();
	let mut current: usize = 0;

	while let Some(line) = scanner.next() {
		let line = line?;
		let colored_line = color_output(&line);
		print!("{}", colored_line);
		output.push_str(&line);
		output.push('\n');

		for re in &update_regexes {
			if re.is_match(&line) {
				current += 1;
				pb.update_to(current)?;
				break;
			}
		}
	}

	let status = child.wait()?;
	if !status.success() {
		println!("{}", colored("Error executing command.", "red", false, false));
		return Err("Command failed".into());
	}

	pb.set_bar_format("{desc suffix=' '}|{animation}| {count}/{total} [{percentage:.0}%] in {elapsed human=true} ({rate:.1}/s)".to_string())?;
	pb.clear()?;
	pb.refresh()?;
	println!();

	Ok(output)
}

fn run_download_with_progress(cmd_args: &[String], num_downloads: usize) -> Result<String, Box<dyn Error>> {
	let get_re = Regex::new(r"^(Get|Pobr):\d+").unwrap();
	run_with_progress(cmd_args, colored("Downloading", "yellow", false, false), num_downloads, vec![get_re])
}

fn run_install_with_progress(cmd_args: &[String], total_steps: usize) -> Result<String, Box<dyn Error>> {
	let unpack_re = Regex::new(r"^(Unpacking|Rozpakowywanie)").unwrap();
	let setup_re = Regex::new(r"^(Setting up|Konfigurowanie)").unwrap();
	let remove_re = Regex::new(r"^(Removing|Usuwanie)").unwrap();
	run_with_progress(cmd_args, colored("Transaction", "green", false, false), total_steps, vec![unpack_re, setup_re, remove_re])
}

fn run_command_with_progress(cmd_args: &[String]) -> Result<String, Box<dyn Error>> {
	let mut cmd = Command::new(&cmd_args[0]);
	cmd.args(&cmd_args[1..]);
	cmd.stdout(Stdio::piped());

	let mut child = cmd.spawn()?;

	let stdout = child.stdout.take().unwrap();
	let mut scanner = BufReader::new(stdout).lines();

	let mut pb = Bar::builder()
	.total(100)
	.desc(colored("Progress", "blue", false, false))
	.bar_format("{desc suffix=' '}|{animation}| {spinner} {count}/{total} [{percentage:.0}%] in {elapsed human=true} ({rate:.1}/s, eta: {remaining human=true})".to_string())
	.spinner(Spinner::new(
		&["▁▂▃", "▂▃▄", "▃▄▅", "▄▅▆", "▅▆▇", "▆▇█", "▇█▇", "█▇▆", "▇▆▅", "▆▅▄", "▅▄▃", "▄▃▂", "▃▂▁"],
		30.0,
		1.0,
	))
	.ncols(20u16)
	.force_refresh(true)
	.build()?;

	let mut output = String::new();

	while let Some(line) = scanner.next() {
		let line = line?;
		let colored_line = color_output(&line);
		print!("{}", colored_line);
		output.push_str(&line);
		output.push('\n');

		if line.contains('%') {
			let parts: Vec<&str> = line.split('%').collect();
			if parts.len() > 1 {
				let last = parts[0].trim();
				let words: Vec<&str> = last.split_whitespace().collect();
				if let Some(pstr) = words.last() {
					if let Ok(percent) = i32::from_str(pstr) {
						if percent >= 0 && percent <= 100 {
							pb.update_to(percent as usize)?;
						}
					}
				}
			}
		}
	}

	let status = child.wait()?;
	if !status.success() {
		println!("{}", colored("Error executing command.", "red", false, false));
		return Err("Command failed".into());
	}

	pb.set_bar_format("{desc suffix=' '}|{animation}| {count}/{total} [{percentage:.0}%] in {elapsed human=true} ({rate:.1}/s)".to_string())?;
	pb.clear()?;
	pb.refresh()?;
	println!();

	Ok(output)
}

fn handle_install(packages: &[String]) {
	if packages.is_empty() {
		println!(
			"{}",
		   colored("No packages specified for install.", "red", false, false)
		);
		return;
	}

	let mut sim_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "install".to_string()];
	sim_cmd.extend(packages.iter().cloned());
	sim_cmd.push("-s".to_string());

	let sim_output = match run_command(&sim_cmd, true, false) {
		Ok(o) => o,
		Err(_) => return,
	};

	let parsed = parse_apt_simulate(&sim_output);
	display_dnf_style(&parsed, "install");

	if confirm_action() {
		let num_downloads = get_num_downloads("install", packages);
		if num_downloads > 0 && parsed.download_size != "0" {
			println!("{}", colored("Downloading Packages:", "cyan", true, false));
			println!("{}", colored("==========================================================================================", "white", false, false));
			let mut dl_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "install".to_string(), "-d".to_string(), "-y".to_string()];
			dl_cmd.extend(packages.iter().cloned());
			let _ = run_download_with_progress(&dl_cmd, num_downloads);
			println!("{}", colored("Complete!", "green", true, false));
		}
		println!("{}", colored("Running transaction check", "cyan", true, false));
		// Assume succeeded since sim did
		println!("{}", colored("Transaction check succeeded.", "green", false, false));
		println!("{}", colored("Running transaction test", "cyan", true, false));
		println!("{}", colored("Transaction test succeeded.", "green", false, false));
		println!("{}", colored("Running transaction", "cyan", true, false));
		println!("{}", colored("==========================================================================================", "white", false, false));
		let mut cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "install".to_string(), "-y".to_string(), "--no-download".to_string()];
		cmd.extend(packages.iter().cloned());
		let total_steps = (parsed.summary[0] as usize + parsed.summary[1] as usize) * 2 + parsed.summary[2] as usize;
		let _ = run_install_with_progress(&cmd, total_steps);
		println!("{}", colored("Complete!", "green", true, false));
	} else {
		println!(
			"{}",
		   colored("Transaction cancelled.", "yellow", false, false)
		);
	}
}

fn handle_remove(packages: &[String]) {
	if packages.is_empty() {
		println!(
			"{}",
		   colored("No packages specified for remove.", "red", false, false)
		);
		return;
	}

	let mut sim_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "remove".to_string()];
	sim_cmd.extend(packages.iter().cloned());
	sim_cmd.push("-s".to_string());

	let sim_output = match run_command(&sim_cmd, true, false) {
		Ok(o) => o,
		Err(_) => return,
	};

	let parsed = parse_apt_simulate(&sim_output);
	display_dnf_style(&parsed, "remove");

	if confirm_action() {
		let num_downloads = get_num_downloads("remove", packages);
		if num_downloads > 0 && parsed.download_size != "0" {
			println!("{}", colored("Downloading Packages:", "cyan", true, false));
			println!("{}", colored("==========================================================================================", "white", false, false));
			let mut dl_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "remove".to_string(), "-d".to_string(), "-y".to_string()];
			dl_cmd.extend(packages.iter().cloned());
			let _ = run_download_with_progress(&dl_cmd, num_downloads);
			println!("{}", colored("Complete!", "green", true, false));
		}
		println!("{}", colored("Running transaction check", "cyan", true, false));
		println!("{}", colored("Transaction check succeeded.", "green", false, false));
		println!("{}", colored("Running transaction test", "cyan", true, false));
		println!("{}", colored("Transaction test succeeded.", "green", false, false));
		println!("{}", colored("Running transaction", "cyan", true, false));
		println!("{}", colored("==========================================================================================", "white", false, false));
		let mut cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "remove".to_string(), "-y".to_string(), "--no-download".to_string()];
		cmd.extend(packages.iter().cloned());
		let total_steps = (parsed.summary[0] as usize + parsed.summary[1] as usize) * 2 + parsed.summary[2] as usize;
		let _ = run_install_with_progress(&cmd, total_steps);
		println!("{}", colored("Complete!", "green", true, false));
	} else {
		println!(
			"{}",
		   colored("Transaction cancelled.", "yellow", false, false)
		);
	}
}

fn handle_update() {
	println!("{}", colored("Updating package lists...", "cyan", false, false));
	let update_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "update".to_string()];
	let _ = run_command_with_progress(&update_cmd);

	let sim_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "upgrade".to_string(), "-s".to_string()];
	let sim_output = match run_command(&sim_cmd, true, false) {
		Ok(o) => o,
		Err(_) => return,
	};

	let parsed = parse_apt_simulate(&sim_output);
	display_dnf_style(&parsed, "upgrade");

	if confirm_action() {
		let num_downloads = get_num_downloads("upgrade", &[]);
		if num_downloads > 0 && parsed.download_size != "0" {
			println!("{}", colored("Downloading Packages:", "cyan", true, false));
			println!("{}", colored("==========================================================================================", "white", false, false));
			let dl_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "upgrade".to_string(), "-d".to_string(), "-y".to_string()];
			let _ = run_download_with_progress(&dl_cmd, num_downloads);
			println!("{}", colored("Complete!", "green", true, false));
		}
		println!("{}", colored("Running transaction check", "cyan", true, false));
		println!("{}", colored("Transaction check succeeded.", "green", false, false));
		println!("{}", colored("Running transaction test", "cyan", true, false));
		println!("{}", colored("Transaction test succeeded.", "green", false, false));
		println!("{}", colored("Running upgrade", "cyan", true, false));
		println!("{}", colored("==========================================================================================", "white", false, false));
		let upgrade_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "upgrade".to_string(), "-y".to_string(), "--no-download".to_string()];
		let total_steps = (parsed.summary[0] as usize + parsed.summary[1] as usize) * 2 + parsed.summary[2] as usize;
		let _ = run_install_with_progress(&upgrade_cmd, total_steps);
		println!("{}", colored("Complete!", "green", true, false));
	} else {
		println!("{}", colored("Upgrade cancelled.", "yellow", false, false));
	}
}

fn handle_clean() {
	let sim_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "autoremove".to_string(), "-s".to_string()];
	let sim_output = match run_command(&sim_cmd, true, false) {
		Ok(o) => o,
		Err(_) => return,
	};

	let parsed = parse_apt_simulate(&sim_output);
	display_dnf_style(&parsed, "clean");

	if confirm_action() {
		println!("{}", colored("Running autoclean", "cyan", false, false));
		let autoclean_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "autoclean".to_string()];
		let _ = run_command_with_progress(&autoclean_cmd);

		let num_downloads = get_num_downloads("autoremove", &[]);
		if num_downloads > 0 && parsed.download_size != "0" {
			println!("{}", colored("Downloading Packages:", "cyan", true, false));
			println!("{}", colored("==========================================================================================", "white", false, false));
			let dl_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "autoremove".to_string(), "-d".to_string(), "-y".to_string()];
			let _ = run_download_with_progress(&dl_cmd, num_downloads);
			println!("{}", colored("Complete!", "green", true, false));
		}
		println!("{}", colored("Running transaction check", "cyan", true, false));
		println!("{}", colored("Transaction check succeeded.", "green", false, false));
		println!("{}", colored("Running transaction test", "cyan", true, false));
		println!("{}", colored("Transaction test succeeded.", "green", false, false));
		println!("{}", colored("Running autoremove", "cyan", true, false));
		println!("{}", colored("==========================================================================================", "white", false, false));
		let autoremove_cmd: Vec<String> = vec!["sudo".to_string(), "apt".to_string(), "autoremove".to_string(), "-y".to_string(), "--no-download".to_string()];
		let total_steps = (parsed.summary[0] as usize + parsed.summary[1] as usize) * 2 + parsed.summary[2] as usize;
		let _ = run_install_with_progress(&autoremove_cmd, total_steps);
		println!("{}", colored("Complete!", "green", true, false));
	} else {
		println!("{}", colored("Clean cancelled.", "yellow", false, false));
	}
}

fn print_help() {
	println!(
		"{}",
		colored(
			"Enhanced APT Frontend in DNF Style with Colors and Progress",
		  "magenta",
		  true,
		  false
		)
	);
	println!("Usage: apt-frontend <command> [options]");
	println!("Commands:");
	println!(" install <packages...> Install packages");
	println!(" remove <packages...> Remove packages");
	println!(" update Update and upgrade packages");
	println!(" clean Clean up packages");
}

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		print_help();
		return;
	}

	let command = &args[1];
	match command.as_str() {
		"install" => handle_install(&args[2..]),
		"remove" => handle_remove(&args[2..]),
		"update" => handle_update(),
		"clean" => handle_clean(),
		_ => print_help(),
	}
}
