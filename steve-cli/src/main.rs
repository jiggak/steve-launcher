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

mod cli;
mod cmds;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::{io, path::{Path, PathBuf}};

use cmds::{
    clear_credentials, create_instance, launch_instance, msal_login,
    modpack_search_and_install, modpack_zip_install, print_account_status
};
use cli::{AuthCommands, Parser, Cli, Commands};
use steve::{env, Progress};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(dir) = cli.data_dir {
        env::set_data_dir(dir.to_str().unwrap());
    }

    match cli.command {
        Commands::Create { dir, mc_version, snapshots, forge } => {
            let instance_dir = absolute_path(&dir)?;

            create_instance(&instance_dir, mc_version, snapshots, forge).await
        },
        Commands::Launch { dir, detach } => {
            let instance_dir = absolute_path(&dir)?;

            launch_instance(&instance_dir, detach).await
        },
        Commands::Auth { command } => {
            if let Some(command) = command {
                match command {
                    AuthCommands::Status => print_account_status(),
                    AuthCommands::Clear => clear_credentials()
                }
            } else {
                msal_login().await
            }
        },
        Commands::Import { dir, zip_file } => {
            let instance_dir = absolute_path(&dir)?;

            modpack_zip_install(&instance_dir, &zip_file).await
        },
        Commands::Modpack { dir, search, search_limit } => {
            let instance_dir = absolute_path(&dir)?;

            modpack_search_and_install(&instance_dir, &search, search_limit).await
        },
        Commands::Completion => {
            Ok(print!("{}", include_str!("../steve-completion.bash")))
        }
    }
}

fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    Ok(if !path.is_absolute() {
        std::env::current_dir()?.join(path)
    } else {
        path.to_owned()
    })
}

struct ProgressHandler {
    progress: ProgressBar
}

impl ProgressHandler {
    fn new() -> Self {
        ProgressHandler {
            progress: ProgressBar::with_draw_target(None, ProgressDrawTarget::stdout())
                .with_style(ProgressStyle::with_template("{bar:40.cyan/blue} {msg} {pos}/{len}").unwrap())
        }
    }
}

impl Progress for ProgressHandler {
    fn advance(&mut self, current: usize) {
        self.progress.set_position(current as u64);
    }

    fn begin(&mut self, message: &'static str, total: usize) {
        self.progress.set_length(total as u64);
        self.progress.set_message(message);
        self.progress.reset();
    }

    fn end(&mut self) {
        self.progress.finish_and_clear();
    }
}
