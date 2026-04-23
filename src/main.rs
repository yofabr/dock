mod archiver;
mod config;

use clap::{Parser, Subcommand};
use config::{generate_id, Archive, Config};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
use time::UtcDateTime;

const RESET: &str = "\x1b[0m";
const ERROR: &str = "\x1b[1;38;5;196m";
const SUCCESS: &str = "\x1b[1;38;5;46m";
const WARNING: &str = "\x1b[1;38;5;226m";
const INFO: &str = "\x1b[38;5;75m";
const HEADER: &str = "\x1b[1;38;5;141m";
const HIGHLIGHT: &str = "\x1b[38;5;117m";
const DIM: &str = "\x1b[38;5;245m";
const PROMPT: &str = "\x1b[1;38;5;81m";

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
            eprintln!("{}Error: {}{}", ERROR, e, RESET);
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
                    print!("{}Archive name: {}", PROMPT, RESET);
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let name = input.trim().to_string();
                    match Archive::validate_name(&name) {
                        Ok(name) if !config.archives.iter().any(|a| a.name == name) => name,
                        Ok(_) => {
                            eprintln!("{}Error: name already exists{}", ERROR, RESET);
                            return Ok(());
                        }
                        Err(e) => {
                            eprintln!("{}Error: {}{}", ERROR, e, RESET);
                            return Ok(());
                        }
                    }
                }
            };

            if config.archives.iter().any(|a| a.name == archive_name) {
                print!("{}Directory '{}' exists. Overwrite? [y/N]: {}", WARNING, archive_name, RESET);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if !input.trim().to_lowercase().starts_with('y') {
                    println!("{}Aborted.{}", DIM, RESET);
                    return Ok(());
                }
            }

            let archive_id = generate_id();
            let target_path = config
                .archive_path
                .join(format!("{}_{}.tar.gz", archive_id, archive_name));

            println!("{}Archiving '{}'...{}", INFO, source.display(), RESET);

            print!("{}  Compressing... {}", PROMPT, RESET);
            io::stdout().flush()?;

            let size = archiver::create_tar_gz(&source, &target_path)?;

            println!("{}Done!{}", SUCCESS, RESET);

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
            println!("{}Archived '{}' ({}){}", SUCCESS, archive_name, format_size(size), RESET);
        }
        Commands::Mount { archive: archive_arg, target_dir } => {
            let found = config
                .archives
                .iter()
                .find(|a| a.id == *archive_arg || a.name == *archive_arg);

            match found {
                Some(archive) => {
                    let base_target = PathBuf::from(target_dir);
                    let project_target = base_target.join(&archive.name);

                    if project_target.exists() {
                        print!("{}Directory '{}' exists. Overwrite? [y/N]: {}", WARNING, project_target.display(), RESET);
                        io::stdout().flush()?;
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        if !input.trim().to_lowercase().starts_with('y') {
                            println!("{}Aborted.{}", DIM, RESET);
                            return Ok(());
                        }
                        std::fs::remove_dir_all(&project_target)?;
                    }

                    println!("{}Extracting {}...{}", INFO, archive.name, RESET);

                    print!("{}  Extracting... {}", PROMPT, RESET);
                    io::stdout().flush()?;

                    archiver::extract_tar_gz(&archive.path, &project_target)?;

                    println!("{}Done!{}", SUCCESS, RESET);
                    println!("{}Mounted at: {}{}", SUCCESS, project_target.display(), RESET);
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
            println!("{}Archive storage: {}{}", INFO, config.archive_path.display(), RESET);
            println!("{}Archives ({}){}:", HEADER, config.archives.len(), RESET);

            if config.archives.is_empty() {
                println!("  {}No archives yet.{}", DIM, RESET);
            } else {
                println!();
                for archive in &config.archives {
                    println!(
                        "  {} {:30} {}({})",
                        HIGHLIGHT, archive.name, INFO, archive.id
                    );
                }
            }
        }
        Commands::Delete { archive } => {
            if let Some(idx) = config.archives.iter().position(|a| a.id == *archive || a.name == *archive) {
                let deleted = config.archives.remove(idx);
                if deleted.path.exists() {
                    std::fs::remove_file(&deleted.path)?;
                }
                config.save()?;
                println!("{}Deleted '{}' ({}){}", SUCCESS, deleted.name, format_size(deleted.size), RESET);
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Archive '{}' not found", archive),
                ));
            }
        }
        Commands::Config => {
            println!("{}Current configuration:{}", HEADER, RESET);
            println!("  {}Archive storage: {}{}", INFO, config.archive_path.display(), RESET);
            println!("  {}Archives count: {}{}", INFO, config.archives.len(), RESET);
        }
    }

    Ok(())
}