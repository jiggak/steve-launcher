mod cli;

use dialoguer::Select;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

use cli::{Parser, Cli, Commands};
use mcli::commands::{Progress, create_instance, get_forge_versions, launch_instance, login};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { dir, mc_version, forge } => {
            let instance_dir = match dir.is_absolute() {
                false => std::env::current_dir()?.join(dir),
                true => dir
            };

            let forge_version = if let Some(forge_version) = forge {
                if forge_version == "prompt" {
                    Some(prompt_forge_version(&mc_version).await?)
                } else {
                    Some(forge_version)
                }
            } else {
                None
            };

            create_instance(&instance_dir, &mc_version, forge_version).await
        },
        Commands::Launch { dir } => {
            let mut progress = ProgressHandler::new();
            launch_instance(&dir, &mut progress).await
        },
        Commands::Auth => {
            login(|url, code| {
                println!("Open the URL in your browser and enter the code: {code}\n\t{url}");
            }).await
        }
    }
}

async fn prompt_forge_version(mc_version: &String) -> Result<String, Box<dyn std::error::Error>> {
    let versions = get_forge_versions(mc_version).await?;

    let recommend_index = versions.iter()
        .position(|v| v.recommended)
        .unwrap_or(0);

    let items: Vec<_> = versions.iter()
        .map(|v| match v.recommended {
            false => v.version.clone(),
            true => format!("{ver} *", ver = v.version)
        })
        .collect();

    let selection = Select::new()
        .with_prompt("Select Forge version (* recommended version)")
        .items(&items)
        .default(recommend_index)
        .interact()?;

    Ok(versions[selection].version.clone())
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
