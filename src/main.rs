mod cli;

use cli::{Parser, Cli, Commands};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use mcli::commands::{Progress, create_instance, launch_instance};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { dir, mc_version } => {
            let instance_dir = match dir.is_absolute() {
                false => std::env::current_dir()?.join(dir),
                true => dir
            };

            create_instance(&instance_dir, &mc_version).await
        },
        Commands::Launch { dir } => {
            let mut handler = ProgressHandler::new();
            launch_instance(&dir, &mut handler).await
        }
    }
}

struct ProgressHandler {
    progress: ProgressBar
}

impl ProgressHandler {
    fn new() -> Self {
        ProgressHandler {
            progress: ProgressBar::with_draw_target(None, ProgressDrawTarget::stdout())
                .with_style(ProgressStyle::with_template("{bar:40.cyan/blue} {msg}").unwrap())
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
