pub use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create instance directory and manifest file
    Create {
        /// Path to directory of new instance
        dir: PathBuf,
        /// Version of minecraft
        mc_version: String
    },

    /// Download instance assets and launch
    Launch {
        /// Path to directory of instance
        dir: PathBuf
    },

    Auth
}
