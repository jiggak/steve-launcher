/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2023 Josh Kropf <josh@slashdev.ca>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

pub use clap::Parser;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Set the directory where shared instance data is stored
    /// [default: $STEVE_DATA_HOME or $XDG_DATA_HOME/steve]
    #[arg(short, verbatim_doc_comment)]
    pub data_dir: Option<PathBuf>,

    /// Instance directory with manifest and game files, defaults to "."
    #[arg(short, global = true)]
    pub instance_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create instance directory and manifest file
    Create {
        /// Version of minecraft or prompt to select from list when not specified
        mc_version: Option<String>,

        /// Enable snapshots in prompt
        #[arg(long)]
        snapshots: bool,

        /// Mod laoder <forge|neoforge>[-<version>], prompt for version when not specified
        #[arg(long)]
        loader: Option<String>
    },

    /// Download instance assets and launch
    Launch {
        /// Allow steve to exit while the java process is running
        #[arg(short)]
        detach: bool
    },

    /// Authenticate with your Microsoft account and save account details
    Auth {
        #[clap(subcommand)]
        command: Option<AuthCommands>
    },

    /// Install CurseForge modpack zip into new or existing instance
    Import {
        /// Path to CurseForge modpack zip
        zip_file: PathBuf
    },

    /// Search and install FTB or CurseForge modpack into new or existing instance
    Modpack {
        /// Modpack search term
        search: String,

        /// Maximum number of search results
        #[arg(short, long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..50))]
        search_limit: u8
    },

    Server {
        #[clap(subcommand)]
        command: ServerCommands
    },

    /// Output bash completion code
    ///
    /// eval "$(steve completion)"
    Completion
}

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Print information about the stored account details
    Status,

    /// Delete stored account details
    Clear
}

#[derive(Subcommand)]
pub enum ServerCommands {
    New {
        /// Version of minecraft or prompt to select from list when not specified
        mc_version: Option<String>,

        /// Mod laoder <forge|neoforge>[-<version>], prompt for version when not specified
        #[arg(long)]
        loader: Option<String>
    },

    Modpack(ServerModpackArgs),

    Launch
}

#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct ServerModpackArgs {
    #[arg(long)]
    pub ftb: Option<u32>,

    pub search_term: Option<String>
}
