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
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Set the directory where shared instance data is stored
    /// [default: $STEVE_DATA_HOME or $XDG_DATA_HOME/steve]
    #[arg(short, verbatim_doc_comment)]
    pub data_dir: Option<PathBuf>,

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
    },

    /// Search and install FTB or CurseForge modpack into new or existing instance
    Modpack {
        /// Path to instance directory
        dir: PathBuf,

        /// Modpack search term
        search: String,

        /// Maximum number of search results
        #[arg(short, long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..50))]
        search_limit: u8
    }
}
