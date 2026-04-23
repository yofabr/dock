mod archiver;
mod config;

use clap::{Parser, Subcommand};
use config::{generate_id, Archive, Config};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
use time::UtcDateTime;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const PURPLE: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const GRAY: &str = "\x1b[90m";

fn c(s: &str) -> String {
    format!("{}{}{}", CYAN, s, RESET)
}

fn r(s: &str) -> String {
    format!("{}{}{}", RED, s, RESET)
}

fn g(s: &str) -> String {
    format!("{}{}{}", GREEN, s, RESET)
}

fn y(s: &str) -> String {
    format!("{}{}{}", YELLOW, s, RESET)
}

fn b(s: &str) -> String {
    format!("{}{}{}", BLUE, s, RESET)
}

fn p(s: &str) -> String {
    format!("{}{}{}", PURPLE, s, RESET)
}

fn gr(s: &str) -> String {
    format!("{}{}{}", GRAY, s, RESET)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_date(time: &UtcDateTime) -> String {
    let year = time.year();
    let month: u8 = time.month().into();
    let day = time.day();
    let hour = time.hour();
    let minute = time.minute();
    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, month, day, hour, minute)
}

#[derive(Debug, Parser)]
#[command(name = "dock")]
#[command(about = "dock: archive and mount your projects easily", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Archive { path: String },
    Mount { archive: String, target_dir: String },
    List,
    Delete { archive: String },
    Config,
}

fn main() {
    let exit_code = match run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{}Error: {}{}", r("Error:"), e, RESET);
            1
        }
    };
    process::exit(exit_code);
}

fn run() -> std::io::Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load_or_create()?;

    match &cli.command {
        Commands::Archive { path } => {
            let source = if path == "." || path == ".." {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else {
                PathBuf::from(&path)
            };

            if !source.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Path '{}' does not exist", path),
                ));
            }

            let archive_name = source
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| Archive::validate_name(s).ok())
                .filter(|n: &String| !n.is_empty());

            let archive_name = match archive_name {
                Some(name) => name,
                None => {
                    print!("{}Archive name: {}", c("?"), RESET);
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let name = input.trim().to_string();
                    match Archive::validate_name(&name) {
                        Ok(name) if !config.archives.iter().any(|a| a.name == name) => name,
                        Ok(_) => {
                            eprintln!("{}name already exists", r("Error:"));
                            return Ok(());
                        }
                        Err(e) => {
                            eprintln!("{}{}", r("Error:"), e);
                            return Ok(());
                        }
                    }
                }
            };

            if config.archives.iter().any(|a| a.name == archive_name) {
                print!("{} '{}' exists. Overwrite? [y/N]: {}", y("!"), archive_name, RESET);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if !input.trim().to_lowercase().starts_with('y') {
                    println!("{}Aborted.", gr(">"));
                    return Ok(());
                }
            }

            let archive_id = generate_id();
            let target_path = config
                .archive_path
                .join(format!("{}_{}.tar.gz", archive_id, archive_name));

            println!("{} Archiving '{}'...", c(">>"), source.display());

            print!("{} Compressing... ", c(".."));
            io::stdout().flush()?;

            let size = archiver::create_tar_gz(&source, &target_path)?;

            println!("{}", g("OK"));

            config.archives.retain(|a| a.name != archive_name);
            config.archives.push(Archive {
                id: archive_id,
                name: archive_name.clone(),
                path: target_path,
                created_at: UtcDateTime::now(),
                size,
                items_count: 0,
            });

            config.save()?;
            println!("{} '{}' archived ({})", g("+"), archive_name, format_size(size));
        }
        Commands::Mount { archive: archive_arg, target_dir } => {
            let found = config
                .archives
                .iter()
                .find(|a| a.id == *archive_arg || a.name == *archive_arg);

            match found {
                Some(archive) => {
                    let mut base_target = PathBuf::from(target_dir);
                    let mut folder_name = archive.name.clone();

                    while base_target.join(&folder_name).exists() {
                        print!("{}Folder '{}' already exists. Enter new name: {}", y("!"), folder_name, RESET);
                        io::stdout().flush()?;
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        let new_name = input.trim().to_string();
                        if new_name.is_empty() {
                            println!("{}Aborted.", gr(">"));
                            return Ok(());
                        }
                        folder_name = new_name;
                    }

                    let project_target = base_target.join(&folder_name);

                    println!("{} Extracting {}...", c(">>"), folder_name);

                    print!("{} Extracting... ", c(".."));
                    io::stdout().flush()?;

                    archiver::extract_tar_gz(&archive.path, &project_target)?;

                    println!("{}", g("OK"));
                    println!("{} Mounted at: {}", g("+"), project_target.display());
                }
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Archive '{}' not found", archive_arg),
                    ));
                }
            }
        }
        Commands::List => {
            println!("{}Archive storage:", b("::"));
            println!("  {}", config.archive_path.display());
            println!();

            if config.archives.is_empty() {
                println!("{}No archives yet.", gr("::"));
            } else {
                println!("{}ARCHIVE           SIZE           CREATED", p("::"));
                println!("{}", gr("------------------------------------------------"));
                for archive in &config.archives {
                    let name = format!("{:<16}", archive.name);
                    let size = format!("{:>12}", format_size(archive.size));
                    let date = format_date(&archive.created_at);
                    println!("{}{} {}", p(&name), b(&size), gr(&date));
                }
                println!();
                let total_size: u64 = config.archives.iter().map(|a| a.size).sum();
                println!("{}Total: {} archives ({})", b("::"), config.archives.len(), format_size(total_size));
            }
        }
        Commands::Delete { archive } => {
            if let Some(idx) = config.archives.iter().position(|a| a.id == *archive || a.name == *archive) {
                let deleted = config.archives.remove(idx);
                if deleted.path.exists() {
                    std::fs::remove_file(&deleted.path)?;
                }
                config.save()?;
                println!("{} Deleted '{}' ({})", g("-"), deleted.name, format_size(deleted.size));
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Archive '{}' not found", archive),
                ));
            }
        }
        Commands::Config => {
            println!("{}Configuration:", p("::"));
            println!("  {}Archive storage: {}", b("::"), config.archive_path.display());
            println!("  {}Archives: {}", b("::"), config.archives.len());
        }
    }

    Ok(())
}