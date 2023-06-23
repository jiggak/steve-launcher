mod cli;

use cli::{Parser, Cli, Commands};
use indicatif::ProgressBar;
use mcli::{create_instance, Progress};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { dir, mc_version } => {
            let mut handler = ProgressHandler::new();
            create_instance(&dir, &mc_version, &mut handler).await
        }
    }
}

struct ProgressHandler {
    progress: Option<ProgressBar>
}

impl ProgressHandler {
    fn new() -> Self {
        ProgressHandler {
            progress: None
        }
    }
}

impl Progress for ProgressHandler {
    fn advance(&mut self, current: usize) {
        self.progress.as_ref().unwrap().set_position(current as u64);
    }

    fn begin(&mut self, total: usize) {
        self.progress = Some(ProgressBar::new(total as u64));
    }
}
