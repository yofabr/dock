mod archiver;
mod config;

use clap::{Parser, Subcommand};
use config::{generate_id, Archive, Config};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
use time::UtcDateTime;

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

fn main() {
    let exit_code = match run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {}", e);
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
                    print!("Archive name: ");
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let name = input.trim().to_string();
                    match Archive::validate_name(&name) {
                        Ok(name) if !config.archives.iter().any(|a| a.name == name) => name,
                        Ok(_) => {
                            eprintln!("Error: name already exists");
                            return Ok(());
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            return Ok(());
                        }
                    }
                }
            };

            if config.archives.iter().any(|a| a.name == archive_name) {
                print!("Archive '{}' already exists. Overwrite? [y/N]: ", archive_name);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if !input.trim().to_lowercase().starts_with('y') {
                    println!("Aborted.");
                    return Ok(());
                }
            }

            let archive_id = generate_id();
            let target_path = config
                .archive_path
                .join(format!("{}_{}.tar.gz", archive_id, archive_name));

            println!(
                "Archiving '{}' to '{}'",
                source.display(),
                target_path.display()
            );

            let size = archiver::create_tar_gz(&source, &target_path)?;

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
            println!("Successfully archived '{}'", path);
        }
        Commands::Mount {
            archive: archive_arg,
            target_dir,
        } => {
            let found = config
                .archives
                .iter()
                .find(|a| a.id == *archive_arg || a.name == *archive_arg);

            match found {
                Some(archive) => {
                    let base_target = PathBuf::from(target_dir);
                    let project_target = base_target.join(&archive.name);

                    if project_target.exists() {
                        print!("Directory '{}' exists. Overwrite? [y/N]: ", project_target.display());
                        io::stdout().flush()?;
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        if !input.trim().to_lowercase().starts_with('y') {
                            println!("Aborted.");
                            return Ok(());
                        }
                        std::fs::remove_dir_all(&project_target)?;
                    }

                    println!("Extracting {} ({} bytes)...", archive.name, archive.size);
                    archiver::extract_tar_gz(&archive.path, &project_target)?;
                    println!("Done. Mounted at: {}", project_target.display());
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
            println!(
                "Archive storage: {}",
                config.archive_path.display()
            );
            println!("Archives ({}):\n", config.archives.len());
            for archive in &config.archives {
                println!(
                    "  {:15}  {:>8} bytes",
                    archive.name,
                    archive.size
                );
            }
        }
        Commands::Delete { archive } => {
            if let Some(idx) = config.archives.iter().position(|a| a.id == *archive || a.name == *archive) {
                let deleted = config.archives.remove(idx);
                if deleted.path.exists() {
                    std::fs::remove_file(&deleted.path)?;
                }
                config.save()?;
                println!("Deleted archive '{}' ({} bytes)", deleted.name, deleted.size);
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Archive '{}' not found", archive),
                ));
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