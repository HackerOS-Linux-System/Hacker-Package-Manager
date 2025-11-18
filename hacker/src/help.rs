use colored::*;
use std::fs;
pub fn display_ascii() {
    match fs::read_to_string("/usr/share/HackerOS/Config-Files/HackerOS-Ascii") {
        Ok(content) => println!("{}", content.bright_cyan().bold().on_black()),
        Err(_) => println!("{}", "File not found".red().bold().on_black()),
    }
}
