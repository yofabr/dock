mod config;

use clap::{Parser, Subcommand};
use config::{generate_id, Archive, Config};
use std::io::{self, Write};
use std::path::PathBuf;
use time::UtcDateTime;

fn prompt_name(exists: impl Fn(&str) -> bool) -> Option<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;
    let input = input.trim().to_string();
    match Archive::validate_name(&input) {
        Ok(valid) if !exists(&valid) => Some(valid),
        Ok(_) => {
            eprintln!("Error: name already exists");
            None
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            None
        }
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
    /// Archive a project directory to the archive store
    Archive {
        /// Path to the project directory
        path: String,
    },
    /// Mount an archived project to a target directory
    Mount {
        /// Name or ID of the archive
        archive: String,
        /// Directory where to mount/extract the project
        target_dir: String,
    },
    /// List all available archives
    List,
    /// Delete an archive
    Delete {
        /// Name or ID of the archive to delete
        archive: String,
    },
    /// Configure or show archive storage location
    Config,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load_or_create()?;

    match &cli.command {
        Commands::Archive { path } => {
            let source = if path == "." {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else {
                PathBuf::from(&path)
            };

            if !source.exists() {
                eprintln!("Error: Path '{}' does not exist", path);
                return Ok(());
            }

            let exists = |name: &str| config.archives.iter().any(|a| a.name == name);

            let archive_name = source
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| Archive::validate_name(s).ok())
                .filter(|n| !n.is_empty() && !exists(n));

            let archive_name = match archive_name {
                Some(name) => name,
                None => {
                    print!("Archive name: ");
                    io::stdout().flush()?;
                    match prompt_name(&exists) {
                        Some(name) => name,
                        None => return Ok(()),
                    }
                }
            };

            let archive_id = generate_id();
            let target_path = config.archive_path.join(format!("{}_{}.tar.gz", archive_id, archive_name));

            println!("Archiving '{}' to '{}'", source.display(), target_path.display());

            config.archives.push(Archive {
                id: archive_id,
                name: archive_name,
                path: target_path,
                created_at: UtcDateTime::now(),
                size: 0,
                items_count: 0,
            });

            config.save()?;
            println!("Successfully archived '{}'", path);
        }
        Commands::Mount { archive: archive_arg, target_dir } => {
            let found = config
                .archives
                .iter()
                .find(|a| a.id == *archive_arg || a.name == *archive_arg);

            match found {
                Some(archive) => {
                    println!(
                        "Mounting archive '{}' to '{}'",
                        archive.name, target_dir
                    );
                    println!("(stub) Would extract '{}' to '{}'", archive.path.display(), target_dir);
                }
                None => {
                    eprintln!("Error: Archive '{}' not found", archive_arg);
                }
            }
        }
        Commands::List => {
            println!("Archive storage location: {}\n", config.archive_path.display());
            println!("Archives ({}):", config.archives.len());
            for archive in &config.archives {
                println!("  [{}] {} ({} bytes)", archive.id, archive.name, archive.size);
            }
        }
        Commands::Delete { archive } => {
            let initial_len = config.archives.len();
            config.archives.retain(|a| a.id != *archive && a.name != *archive);

            if config.archives.len() < initial_len {
                config.save()?;
                println!("Successfully deleted archive '{}'", archive);
            } else {
                eprintln!("Error: Archive '{}' not found", archive);
            }
        }
        Commands::Config => {
            println!("Current configuration:");
            println!("  Archive storage: {}", config.archive_path.display());
            println!("  Archives count: {}", config.archives.len());
        }
    }

    Ok(())
}