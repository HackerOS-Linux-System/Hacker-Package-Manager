use colored::*;
use std::io::{self};
use rand::Rng;
pub fn play_game() {
    loop {
        println!("{}", "========== Welcome to Hacker Adventure! ==========".purple().bold().on_black());
        println!("{}", "You are a fun-loving hacker trying to 'hack' into silly systems for laughs.".cyan().bold().on_black());
        println!("{}", "Choose your adventure level:".cyan().bold().on_black());
        println!("{}", "1. Easy (Coffee Machine Hack - Guess the PIN)".white().bold());
        println!("{}", "2. Medium (Cat Meme Database - Decode the Puzzle)".white().bold());
        println!("{}", "3. Hard (Alien UFO Control - Multi-Step Challenge)".white().bold());
        println!("{}", "4. Expert (Quantum Computer Hack - Advanced Riddles and Guesses)".white().bold());
        println!("{}", "5. Ultimate (Matrix Hack - Math and Logic Puzzles)".white().bold());
        println!("{}", "6. Legendary (Cyber Dragon Battle - Strategy and Luck)".white().bold()); // New level
        println!("{}", "7. Mythical (Time Machine Hack - Historical Riddles)".white().bold()); // New level
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");
        let choice: u32 = match input.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                println!("{}", "Invalid choice! Defaulting to Medium.".red().bold().on_black());
                2
            }
        };
        let mut score = 0;
        let mut won = false;
        match choice {
            1 => {
                println!("{}", "Level 1: Hacking the Office Coffee Machine!".green().bold().on_black());
                println!("{}", "Guess the 4-digit PIN (0000-9999). You have 10 attempts.".cyan().on_black());
                let pin = rand::thread_rng().gen_range(0..10000);
                let mut attempts = 0;
                while attempts < 10 {
                    attempts += 1;
                    println!("{}", format!("Attempt {}/10: Enter PIN:", attempts).yellow().bold().on_black());
                    let mut guess = String::new();
                    io::stdin().read_line(&mut guess).expect("Failed to read line");
                    let guess: u32 = match guess.trim().parse() {
                        Ok(num) => num,
                        Err(_) => continue,
                    };
                    if guess == pin {
                        println!("{}", "Success! Coffee for everyone! +100 points.".green().bold().on_black());
                        score += 100;
                        won = true;
                        break;
                    } else if guess < pin {
                        println!("{}", "Too low! The machine buzzes angrily.".yellow().on_black());
                    } else {
                        println!("{}", "Too high! The machine steams up.".yellow().on_black());
                    }
                }
                if !won {
                    println!("{}", format!("Failed! The PIN was {}. No coffee today.", pin).red().bold().on_black());
                }
            }
            2 => {
                println!("{}", "Level 2: Infiltrating the Cat Meme Database!".green().bold().on_black());
                println!("{}", "Solve the riddles to decode the access key.".cyan().on_black());
                let riddles = vec![
                    ("What has keys but can't open locks?", "keyboard"),
                    ("I'm light as a feather, but the strongest hacker can't hold me for much more than a minute. What am I?", "breath"),
                    ("What do you call a hacker who skips school?", "truant"),
                    ("What gets wetter as it dries?", "towel"),
                    ("I speak without a mouth and hear without ears. I have no body, but I come alive with the wind. What am I?", "echo"),
                    ("What has a ring but no finger?", "phone"),
                    ("What can run but never walks, has a mouth but never talks?", "river"),
                    ("What has one eye but can't see?", "needle"),
                    ("What has a neck but no head?", "bottle"), // New riddle
                    ("What can you catch but not throw?", "cold"), // New riddle
                ];
                for (riddle, answer) in riddles {
                    println!("{}", riddle.magenta().bold().on_black());
                    let mut guess = String::new();
                    io::stdin().read_line(&mut guess).expect("Failed to read line");
                    if guess.trim().to_lowercase() == answer {
                        println!("{}", "Correct! +50 points.".green().on_black());
                        score += 50;
                    } else {
                        println!("{}", format!("Wrong! It was '{}'.", answer).red().on_black());
                    }
                }
                if score >= 300 { // Adjusted for more riddles
                    won = true;
                    println!("{}", "Database hacked! Endless cat memes unlocked!".green().bold().on_black());
                } else {
                    println!("{}", "Access denied! Try harder next time.".red().bold().on_black());
                }
            }
            3 => {
                println!("{}", "Level 3: Taking over an Alien UFO!".green().bold().on_black());
                println!("{}", "Complete all challenges to win.".cyan().on_black());
                let num = rand::thread_rng().gen_range(1..101);
                let mut attempts = 0;
                let mut success = false;
                while attempts < 5 {
                    attempts += 1;
                    println!("{}", "Challenge 1: Guess the alien code (1-100):".yellow().bold().on_black());
                    let mut guess = String::new();
                    io::stdin().read_line(&mut guess).expect("Failed to read line");
                    let guess: i32 = match guess.trim().parse() {
                        Ok(num) => num,
                        Err(_) => continue,
                    };
                    if guess == num {
                        println!("{}", "Code cracked! +100 points.".green().on_black());
                        score += 100;
                        success = true;
                        break;
                    } else if guess < num {
                        println!("{}", "Too low! Aliens chuckle.".yellow().on_black());
                    } else {
                        println!("{}", "Too high! UFO wobbles.".yellow().on_black());
                    }
                }
                if !success {
                    println!("{}", "Challenge failed! UFO escapes.".red().bold().on_black());
                    continue;
                }
                println!("{}", "Challenge 2: Choose your hack path:".yellow().bold().on_black());
                println!("{}", "1. Brute force (risky)".white().bold());
                println!("{}", "2. Stealth mode (safe)".white().bold());
                println!("{}", "3. Quantum tunnel (experimental)".white().bold()); // New option
                let mut choice = String::new();
                io::stdin().read_line(&mut choice).expect("Failed to read line");
                match choice.trim() {
                    "1" => {
                        if rand::thread_rng().gen_bool(0.5) {
                            println!("{}", "Brute force worked! +150 points.".green().on_black());
                            score += 150;
                        } else {
                            println!("{}", "Brute force failed! -50 points.".red().on_black());
                            score -= 50;
                        }
                    }
                    "2" => {
                        println!("{}", "Stealth success! +100 points.".green().on_black());
                        score += 100;
                    }
                    "3" => {
                        if rand::thread_rng().gen_bool(0.3) {
                            println!("{}", "Quantum tunnel success! +200 points.".green().on_black());
                            score += 200;
                        } else {
                            println!("{}", "Quantum failure! -100 points.".red().on_black());
                            score -= 100;
                        }
                    }
                    _ => println!("{}", "Invalid choice! No points.".yellow().on_black()),
                }
                println!("{}", "Final Challenge: What do hackers do at the beach?".magenta().bold().on_black());
                println!("{}", "Hint: It involves waves.".cyan().on_black());
                let mut guess = String::new();
                io::stdin().read_line(&mut guess).expect("Failed to read line");
                if guess.trim().to_lowercase().contains("surf") {
                    println!("{}", "Correct! They surf the web. +200 points.".green().on_black());
                    score += 200;
                    won = true;
                } else {
                    println!("{}", "Wrong! UFO self-destructs.".red().on_black());
                }
            }
            4 => {
                println!("{}", "Level 4: Hacking a Quantum Computer!".green().bold().on_black());
                println!("{}", "Solve advanced riddles and guess the quantum state.".cyan().on_black());
                let riddles = vec![
                    ("I am not alive, but I grow; I don't have lungs, but I need air; I don't have a mouth, but water kills me. What am I?", "fire"),
                    ("What can travel around the world while staying in a corner?", "stamp"),
                    ("What has a head, a tail, is brown, and has no legs?", "penny"),
                    ("What is always in front of you but can’t be seen?", "future"),
                    ("What can you break, even if you never pick it up or touch it?", "promise"),
                    ("I have branches, but no fruit, trunk or leaves. What am I?", "bank"),
                    ("What has many keys but can't open a single lock?", "piano"), // New
                    ("What invention lets you look right through a wall?", "window"), // New
                ];
                for (riddle, answer) in riddles {
                    println!("{}", riddle.magenta().bold().on_black());
                    let mut guess = String::new();
                    io::stdin().read_line(&mut guess).expect("Failed to read line");
                    if guess.trim().to_lowercase() == answer {
                        println!("{}", "Correct! +100 points.".green().on_black());
                        score += 100;
                    } else {
                        println!("{}", format!("Wrong! It was '{}'.", answer).red().on_black());
                    }
                }
                let quantum = rand::thread_rng().gen_range(1..1001);
                println!("{}", "Final Quantum Challenge: Guess if the state is even or odd (number 1-1000).".yellow().bold().on_black());
                let mut guess = String::new();
                io::stdin().read_line(&mut guess).expect("Failed to read line");
                let is_even = quantum % 2 == 0;
                let guessed_even = guess.trim().to_lowercase() == "even";
                if (is_even && guessed_even) || (!is_even && !guessed_even) {
                    println!("{}", "Quantum state hacked! +300 points.".green().on_black());
                    score += 300;
                    won = true;
                } else {
                    println!("{}", format!("Wrong! It was {}. Quantum collapse!", if is_even { "even" } else { "odd" }).red().on_black());
                }
            }
            5 => {
                println!("{}", "Level 5: Ultimate Matrix Hack!".green().bold().on_black());
                println!("{}", "Solve math and logic puzzles to break the matrix.".cyan().on_black());
                // Challenge 1: Math problem
                println!("{}", "Challenge 1: What is 10 + 5 * 2 - 3?".magenta().bold().on_black());
                let mut guess = String::new();
                io::stdin().read_line(&mut guess).expect("Failed to read line");
                if guess.trim() == "17" {
                    println!("{}", "Correct! +150 points.".green().on_black());
                    score += 150;
                } else {
                    println!("{}", "Wrong! It was 17.".red().on_black());
                }
                // Challenge 2: Logic riddle
                println!("{}", "Challenge 2: If a red house is made of red bricks, a blue house of blue bricks, what is a greenhouse made of?".magenta().bold().on_black());
                let mut guess = String::new();
                io::stdin().read_line(&mut guess).expect("Failed to read line");
                if guess.trim().to_lowercase() == "glass" {
                    println!("{}", "Correct! +150 points.".green().on_black());
                    score += 150;
                } else {
                    println!("{}", "Wrong! It's glass.".red().on_black());
                }
                // New Challenge 2.5: Another math
                println!("{}", "Challenge 2.5: What is the square root of 144?".magenta().bold().on_black());
                let mut guess = String::new();
                io::stdin().read_line(&mut guess).expect("Failed to read line");
                if guess.trim() == "12" {
                    println!("{}", "Correct! +100 points.".green().on_black());
                    score += 100;
                } else {
                    println!("{}", "Wrong! It's 12.".red().on_black());
                }
                // Challenge 3: Guess number with more range
                let num = rand::thread_rng().gen_range(1..201);
                let mut attempts = 0;
                let mut success = false;
                while attempts < 6 {
                    attempts += 1;
                    println!("{}", "Challenge 3: Guess the matrix code (1-200):".yellow().bold().on_black());
                    let mut guess = String::new();
                    io::stdin().read_line(&mut guess).expect("Failed to read line");
                    let guess: i32 = match guess.trim().parse() {
                        Ok(num) => num,
                        Err(_) => continue,
                    };
                    if guess == num {
                        println!("{}", "Code cracked! +200 points.".green().on_black());
                        score += 200;
                        success = true;
                        break;
                    } else if guess < num {
                        println!("{}", "Too low! The matrix glitches.".yellow().on_black());
                    } else {
                        println!("{}", "Too high! The code shifts.".yellow().on_black());
                    }
                }
                if !success {
                    println!("{}", "Challenge failed! Matrix resets.".red().bold().on_black());
                } else {
                    won = true;
                }
            }
            6 => {
                println!("{}", "Level 6: Legendary Cyber Dragon Battle!".green().bold().on_black());
                println!("{}", "Defeat the dragon with strategy and luck.".cyan().on_black());
                let mut dragon_hp = 500;
                let mut player_hp = 300;
                while dragon_hp > 0 && player_hp > 0 {
                    println!("{}", format!("Dragon HP: {} | Your HP: {}", dragon_hp, player_hp).yellow().bold().on_black());
                    println!("{}", "Choose action: 1. Attack (50-100 dmg), 2. Hack Shield (reduce dragon dmg), 3. Heal (50-100 hp)".white().bold());
                    let mut choice = String::new();
                    io::stdin().read_line(&mut choice).expect("Failed to read line");
                    match choice.trim() {
                        "1" => {
                            let dmg = rand::thread_rng().gen_range(50..101);
                            dragon_hp -= dmg;
                            println!("{}", format!("You attack! {} damage.", dmg).green().on_black());
                            score += dmg;
                        }
                        "2" => {
                            println!("{}", "You hack the shield! Dragon damage reduced.".green().on_black());
                            score += 50;
                        }
                        "3" => {
                            let heal = rand::thread_rng().gen_range(50..101);
                            player_hp += heal;
                            println!("{}", format!("You heal! +{} HP.", heal).green().on_black());
                        }
                        _ => continue,
                    }
                    let dragon_dmg = rand::thread_rng().gen_range(30..71);
                    player_hp -= dragon_dmg;
                    println!("{}", format!("Dragon attacks! -{} HP.", dragon_dmg).red().on_black());
                }
                if player_hp > 0 {
                    won = true;
                    println!("{}", "Dragon defeated! +500 points.".green().bold().on_black());
                    score += 500;
                } else {
                    println!("{}", "You were defeated by the dragon.".red().bold().on_black());
                }
            }
            7 => {
                println!("{}", "Level 7: Mythical Time Machine Hack!".green().bold().on_black());
                println!("{}", "Solve historical riddles to hack the time machine.".cyan().on_black());
                let riddles = vec![
                    ("Who was the first president of the USA?", "george washington"),
                    ("In what year did World War II end?", "1945"),
                    ("Who invented the telephone?", "alexander graham bell"),
                    ("What ancient wonder was in Egypt?", "pyramids"),
                    ("Who wrote Romeo and Juliet?", "shakespeare"),
                ];
                for (riddle, answer) in riddles {
                    println!("{}", riddle.magenta().bold().on_black());
                    let mut guess = String::new();
                    io::stdin().read_line(&mut guess).expect("Failed to read line");
                    if guess.trim().to_lowercase() == answer {
                        println!("{}", "Correct! +150 points.".green().on_black());
                        score += 150;
                    } else {
                        println!("{}", format!("Wrong! It was '{}'.", answer).red().on_black());
                    }
                }
                if score >= 600 {
                    won = true;
                    println!("{}", "Time machine hacked! Travel through time!".green().bold().on_black());
                } else {
                    println!("{}", "Time lock engaged! Try again.".red().bold().on_black());
                }
            }
            _ => continue,
        }
        println!("{}", format!("Your score: {}", score).blue().bold().on_black());
        if won {
            println!("{}", "You win the level!".green().bold().on_black());
        }
        println!("{}", "Play again? (y/n)".cyan().bold().on_black());
        let mut again = String::new();
        io::stdin().read_line(&mut again).expect("Failed to read line");
        if again.trim().to_lowercase() != "y" {
            break;
        }
    }
    println!("{}", "========== Thanks for playing Hacker Adventure! ==========".purple().bold().on_black());
}
