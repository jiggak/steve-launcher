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
        mc_version: String,

        /// Enable Forge by setting Forge version or prompt to select from version list
        #[arg(long, value_name = "FORGE_VERSION", default_missing_value = "prompt", num_args = 0..=1)]
        forge: Option<String>
    },

    /// Download instance assets and launch
    Launch {
        /// Path to directory of instance
        dir: PathBuf
    },

    /// Authenticate with your Microsoft account and save account details
    Auth,

    /// Install CurseForge modpack zip into new or existing instance
    Import {
        /// Path to instance directory
        dir: PathBuf,

        /// Path to CurseForge modpack zip
        zip_file: PathBuf
    }
}
