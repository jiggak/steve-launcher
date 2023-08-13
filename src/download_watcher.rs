use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::{collections::HashMap, fs, io::Result as IoResult, path::{Path, PathBuf}};

use crate::env;

pub struct DownloadWatcher {
    watch_dir: PathBuf,
    file_state: HashMap<String, bool>
}

impl<'a> DownloadWatcher {
    pub fn new<I>(files: I) -> IoResult<Self>
        where I: Iterator<Item = &'a str>
    {
        let mut watcher = DownloadWatcher {
            watch_dir: env::get_downloads_dir(),
            file_state: files
                .map(|f| (f.to_string(), false))
                .collect()
        };

        for f in fs::read_dir(&watcher.watch_dir)? {
            watcher.on_file_complete(&f?.path())?;
        }

        Ok(watcher)
    }

    pub fn begin_watching(mut self,
        file_ready: impl Fn(&DownloadWatcher, &Path) -> IoResult<()>
    ) -> notify::Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

        watcher.watch(&self.watch_dir, RecursiveMode::NonRecursive)?;

        for res in rx {
            let event = res?;

            // println!("event: {:?}", event);

            for path in event.paths {
                if self.on_file_complete(&path)? {
                    file_ready(&self, &path)?;
                }
            }

            if self.is_all_complete() {
                watcher.unwatch(&self.watch_dir)?;
                break;
            }
        }

        Ok(())
    }

    fn on_file_complete(&mut self, path: &PathBuf) -> IoResult<bool> {
        let path_file_name = path.file_name()
            .and_then(|p| p.to_str())
            .unwrap();

        if let Some(status) = self.file_state.get_mut(path_file_name) {
            *status = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn is_file_complete(&self, file_name: &String) -> bool {
        match self.file_state.get(file_name) {
            Some(v) => *v,
            None => false
        }
    }

    pub fn is_all_complete(&self) -> bool {
        self.file_state.values().all(|v| *v)
    }
}
