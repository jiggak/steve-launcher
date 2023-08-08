use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};

use crate::env;

pub struct DownloadWatcher {
    watch_dir: PathBuf,
    watch_files: Vec<DownloadState>
}

impl DownloadWatcher {
    pub fn new<'a, I>(files: I) -> Self
        where I: Iterator<Item = &'a str>
    {
        DownloadWatcher {
            watch_dir: env::get_downloads_dir(),
            watch_files: files.map(|f| {
                DownloadState {
                    file_name: f.to_string(),
                    complete: false
                }
            }).collect()
        }
    }

    pub fn begin_watching(mut self, file_ready: impl Fn(&Path) -> std::io::Result<()>) -> notify::Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

        watcher.watch(&self.watch_dir, RecursiveMode::NonRecursive)?;

        for res in rx {
            match res {
                Ok(event) => {
                    println!("event: {:?}", event);
                    for path in event.paths {
                        let path_file_name = path.file_name().and_then(|p| p.to_str()).unwrap();

                        if let Some(file) = self.watch_files.iter_mut().find(|f| f.file_name == path_file_name).as_mut() {
                            println!("mark {} complete", file.file_name);
                            file.complete = true;
                            file_ready(&path)?;
                        }
                    }

                    if self.watch_files.iter().all(|f| f.complete) {
                        println!("all files complete, stopping watcher");
                        watcher.unwatch(&self.watch_dir)?;
                        break;
                    }
                },
                Err(e) => println!("watch error: {:?}", e)
            }
        }

        Ok(())
    }
}

struct DownloadState {
    file_name: String,
    complete: bool
}
