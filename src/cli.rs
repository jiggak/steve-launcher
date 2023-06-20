pub use clap::{Parser};
use clap::{Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download and create new instance
    Create {
        /// Path to directory of new instance
        dir: String,
        /// Version of minecraft to download
        mc_version: String
    }
}