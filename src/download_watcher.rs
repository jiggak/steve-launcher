use notify::{Config, Error, RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use std::{
    collections::HashMap, path::PathBuf, sync::mpsc::{self, Sender},
    sync::Arc, sync::Mutex, thread
};

use crate::env;

pub struct DownloadWatcher {
    pub watch_dir: PathBuf,
    file_state: Arc<Mutex<HashMap<String, bool>>>
}

impl<'a> DownloadWatcher {
    pub fn new<I>(files: I) -> Self
        where I: Iterator<Item = &'a str>
    {
        let watch_dir = env::get_downloads_dir();
        let file_state = files
            .map(|f| (f.to_string(), watch_dir.join(f).exists()))
            .collect();

        DownloadWatcher {
            watch_dir,
            file_state: Arc::new(Mutex::new(file_state))
        }
    }

    pub fn watch<'scope, 'env>(&'env self, scope: &'scope thread::Scope<'scope, 'env>, tx: Sender<WatcherMessage>) -> notify::Result<impl Fn()> {
        let (watch_tx, watch_rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(watch_tx.clone(), Config::default())?;

        watcher.watch(&self.watch_dir, RecursiveMode::NonRecursive)?;

        let watch_cancel = move || watch_tx.send(Ok(notify::Event::new(notify::EventKind::Other))).unwrap();

        scope.spawn(move || {
            for result in watch_rx {
                match result {
                    Ok(notify::Event { kind: EventKind::Other, .. }) => break,
                    Ok(event) => {
                        for path in event.paths {
                            if self.on_file_complete(&path) {
                                tx.send(WatcherMessage::FileComplete(path)).unwrap();
                            }
                        }

                        if self.is_all_complete() {
                            tx.send(WatcherMessage::AllComplete).unwrap();
                            break;
                        }
                    },
                    Err(error) => {
                        tx.send(WatcherMessage::Error(error)).unwrap();
                        break;
                    }
                }
            }
        });

        Ok(watch_cancel)
    }

    fn on_file_complete(&self, path: &PathBuf) -> bool {
        let path_file_name = path.file_name()
            .and_then(|p| p.to_str())
            .unwrap();

        let mut file_state = self.file_state.lock().unwrap();
        if let Some(value) = file_state.get_mut(path_file_name) {
            *value = true;
            true
        } else {
            false
        }
    }

    pub fn is_file_complete(&self, file_name: &String) -> bool {
        match self.file_state.lock().unwrap().get(file_name) {
            Some(v) => *v,
            None => false
        }
    }

    pub fn is_all_complete(&self) -> bool {
        self.file_state.lock().unwrap().values().all(|v| *v)
    }
}

pub enum WatcherMessage {
    AllComplete,
    FileComplete(PathBuf),
    // FIXME it's kinda weird having this variant here instead of cli app
    KeyPress(char),
    Error(Error)
}
