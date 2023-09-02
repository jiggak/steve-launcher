mod cli;
mod cmds;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::{
    error::Error as StdError, io, path::{Path, PathBuf}
};

use cmds::{
    create_instance, launch_instance, msal_login, modpack_search_and_install,
    modpack_zip_install
};
use cli::{Parser, Cli, Commands};
use steve::Progress;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn StdError>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { dir, mc_version, forge } => {
            let instance_dir = absolute_path(&dir)?;

            create_instance(&instance_dir, &mc_version, forge).await
        },
        Commands::Launch { dir } => {
            let instance_dir = absolute_path(&dir)?;

            launch_instance(&instance_dir).await
        },
        Commands::Auth => {
            msal_login().await
        },
        Commands::Import { dir, zip_file } => {
            let instance_dir = absolute_path(&dir)?;

            modpack_zip_install(&instance_dir, &zip_file).await
        },
        Commands::Modpack { dir, search } => {
            let instance_dir = absolute_path(&dir)?;

            modpack_search_and_install(&instance_dir, &search).await
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
