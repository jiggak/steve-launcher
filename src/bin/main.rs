mod cli;

use std::{
    error::Error as StdError, io, path::{Path, PathBuf}, process::{Command, Stdio},
    sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{self, Sender}}, thread
};

use console::Term;
use dialoguer::{Confirm, Select};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

use cli::{Parser, Cli, Commands};
use steve::{
    Account, AssetClient, ModPack, DownloadWatcher, FileDownload, Instance,
    Progress, WatcherMessage
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn StdError>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { dir, mc_version, forge } => {
            let instance_dir = absolute_path(&dir)?;

            let forge_version = if let Some(forge_version) = forge {
                if forge_version == "prompt" {
                    Some(prompt_forge_version(&mc_version).await?)
                } else {
                    Some(forge_version)
                }
            } else {
                None
            };

            Instance::create(&instance_dir, &mc_version, forge_version)
                .await.map(|_| ())
        },
        Commands::Launch { dir } => {
            let mut progress = ProgressHandler::new();

            let instance = Instance::load(&dir)?;
            instance.launch(&mut progress)
                .await.map(|_| ())
        },
        Commands::Auth => {
            Account::login(|url, code| {
                println!("Open the URL in your browser and enter the code: {code}\n\t{url}");
            }).await.map(|_| ())
        },
        Commands::Import { dir, zip_file } => {
            let mut progress = ProgressHandler::new();

            let instance_dir = absolute_path(&dir)?;

            let pack = ModPack::load_zip(&zip_file)?;

            let instance = if Instance::exists(&dir) {
                if !prompt_confirm("Instance already exists, are you sure you want to install the pack here?")? {
                    return Ok(())
                }

                let mut instance = Instance::load(&instance_dir)?;

                instance.update_manifest(&pack.manifest)?;

                instance
            } else {
                Instance::create(
                    &instance_dir,
                    &pack.manifest.minecraft.version,
                    pack.manifest.minecraft.get_forge_version()
                ).await?
            };

            let downloads = instance.install_pack(&pack, &mut progress)
                .await?;

            if let Some(downloads) = downloads {
                download_blocked(instance, downloads)?;
            }

            Ok(())
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

fn prompt_confirm<S: Into<String>>(prompt: S) -> io::Result<bool> {
    Confirm::new()
        .with_prompt(prompt)
        .interact()
}

async fn prompt_forge_version(mc_version: &String) -> Result<String, Box<dyn StdError>> {
    let client = AssetClient::new();

    let versions = client.get_forge_versions(mc_version).await?;

    let recommend_index = versions.iter()
        .position(|v| v.recommended)
        .unwrap_or(0);

    let items: Vec<_> = versions.iter()
        .map(|v| match v.recommended {
            false => v.version.to_string(),
            true => format!("{ver} *", ver = v.version)
        })
        .collect();

    let selection = Select::new()
        .with_prompt("Select Forge version (* recommended version)")
        .items(&items)
        .default(recommend_index)
        .interact()?;

    Ok(versions[selection].version.to_string())
}

fn download_blocked(instance: Instance, downloads: Vec<FileDownload>) -> Result<(), Box<dyn StdError>> {
    let watcher = DownloadWatcher::new(
        downloads.iter()
            .map(|f| f.file_name.as_str())
    );

    // copy any downloads already in watch dir
    for f in &downloads {
        if watcher.is_file_complete(&f.file_name) {
            let file_path = watcher.watch_dir.join(&f.file_name);
            instance.install_file(f, &file_path)?;
        }
    }

    if watcher.is_all_complete() {
        return Ok(());
    }

    let term = Term::stdout();
    term.hide_cursor()?;

    term.write_line("Files below must be downloaded manually. Press [o] to open all, [x] to quit.")?;

    print_download_state(&term, &watcher, &downloads)?;

    let (tx, rx) = mpsc::channel();

    thread::scope(|scope| -> io::Result<()> {
        let watch_cancel = watcher.watch(scope, tx.clone()).unwrap();
        let readkey_cancel = readkey_thread(scope, term.clone(), tx);

        while let Ok(msg) = rx.recv() {
            match msg {
                WatcherMessage::FileComplete(file_path) => {
                    let file_name = file_path.file_name().unwrap().to_string_lossy();
                    let file = downloads.iter()
                        .find(|d| d.file_name == file_name)
                        .unwrap();
                    instance.install_file(file, &file_path)?;
                    print_download_state(&term, &watcher, &downloads)?;
                },
                WatcherMessage::AllComplete => {
                    break;
                },
                WatcherMessage::KeyPress(ch) => {
                    match ch {
                        'o' => {
                            open_urls(
                                downloads.iter()
                                    .filter_map(|d| match watcher.is_file_complete(&d.file_name) {
                                        false => Some(d.url.as_str()),
                                        _ => None
                                    })
                            )?;
                        },
                        'x' => {
                            break;
                        },
                        _ => { }
                    }
                },
                WatcherMessage::Error(_) => {
                    break;
                }
            }
        }

        watch_cancel();
        readkey_cancel();

        term.clear_to_end_of_screen()?;
        term.show_cursor()?;

        Ok(())
    })?;

    Ok(())
}

fn print_download_state(term: &Term, watcher: &DownloadWatcher, downloads: &Vec<FileDownload>) -> io::Result<()> {
    for x in downloads {
        let status = match watcher.is_file_complete(&x.file_name) {
            true => "✅", false => "❌"
        };

        term.write_line(
            format!("{status} {url}", url = x.url).as_str()
        )?;
    }

    term.move_cursor_up(downloads.len())?;

    Ok(())
}

fn open_urls<'a, T>(urls: T) -> io::Result<()>
    where T: Iterator<Item = &'a str>
{
    for u in urls {
        Command::new("xdg-open")
            .arg(u)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }

    Ok(())
}

fn readkey_thread<'scope, 'env>(scope: &'scope thread::Scope<'scope, 'env>, term: Term, tx: Sender<WatcherMessage>) -> impl Fn() {
    let stop = Arc::new(AtomicBool::new(false));

    let stop_thread = stop.clone();
    let exit_thread = move || stop.store(true, Ordering::Relaxed);

    scope.spawn(move || -> io::Result<()> {
        while !stop_thread.load(Ordering::Relaxed) {
            let ch = term.read_char()?;
            tx.send(WatcherMessage::KeyPress(ch)).unwrap();
        }

        Ok(())
    });

    exit_thread
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
