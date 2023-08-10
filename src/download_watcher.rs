use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::{fs, path::{Path, PathBuf}};

use crate::env;

pub struct DownloadWatcher {
    watch_dir: PathBuf,
    pub state: Vec<DownloadState>
}

impl DownloadWatcher {
    pub fn new<I>(files: I) -> Self
        where I: Iterator<Item = DownloadState>
    {
        DownloadWatcher {
            watch_dir: env::get_downloads_dir(),
            state: files.collect()
        }
    }

    pub fn begin_watching(mut self,
        file_ready: impl Fn(&DownloadWatcher, &Path) -> std::io::Result<()>
    ) -> notify::Result<()> {
        for f in fs::read_dir(&self.watch_dir)? {
            self.check_file_complete(&f?.path(), &file_ready)?;
        }

        if self.is_complete() {
            return Ok(());
        }

        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

        watcher.watch(&self.watch_dir, RecursiveMode::NonRecursive)?;

        for res in rx {
            let event = res?;

            // println!("event: {:?}", event);

            for path in event.paths {
                self.check_file_complete(&path, &file_ready)?;
            }

            if self.is_complete() {
                watcher.unwatch(&self.watch_dir)?;
                break;
            }
        }

        Ok(())
    }

    fn check_file_complete(
        &mut self, path: &PathBuf,
        file_ready: impl Fn(&DownloadWatcher, &Path) -> std::io::Result<()>
    ) -> std::io::Result<()> {
        let path_file_name = path.file_name()
            .and_then(|p| p.to_str())
            .unwrap();

        if let Some(file) = self.state.iter_mut().find(|f| f.file_name == path_file_name).as_mut() {
            file.complete = true;
            file_ready(&self, &path)?;
        }

        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.state.iter().all(|f| f.complete)
    }
}

pub struct DownloadState {
    pub file_name: String,
    pub url: String,
    pub complete: bool
}

impl DownloadState {
    pub fn new(file_name: String, url: String) -> Self {
        DownloadState { file_name, url, complete: false }
    }
}
