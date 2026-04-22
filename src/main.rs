mod config;

use clap::{Parser, Subcommand};

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
        /// Name of the archive
        archive_name: String,
        /// Directory where to mount/extract the project
        target_dir: String,
    },
    /// Unmount (remove) a mounted project from a directory
    Unmount {
        /// Directory to unmount
        target_dir: String,
    },
    /// List all available archives
    List,
    /// Delete an archive
    Delete {
        /// Name of the archive to delete
        archive_name: String,
    },
    /// Configure or show archive storage location
    Config,
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Archive { path } => {
            println!("(stub) Would archive path: {}", path);
        }
        Commands::Mount { archive_name, target_dir } => {
            println!("(stub) Would mount archive '{}' to '{}'", archive_name, target_dir);
        }
        Commands::Unmount { target_dir } => {
            println!("(stub) Would unmount directory: {}", target_dir);
        }
        Commands::List => {
            println!("(stub) Would list all archives");
        }
        Commands::Delete { archive_name } => {
            println!("(stub) Would delete archive: {}", archive_name);
        }
        Commands::Config => {
            println!("(stub) Would configure or show archive storage location");
        }
    }
}

