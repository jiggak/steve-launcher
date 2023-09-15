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

use notify::{Config, Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashMap, path::{Path, PathBuf}, sync::mpsc::{self, Sender},
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
        let watch_cancel = move || {
            let _ = watch_tx.send(Ok(Event::new(EventKind::Other)));
        };

        scope.spawn(move || -> notify::Result<()> {
            // starting watcher inside thread so that it doesn't get dropped
            // from parent scope when method returns
            watcher.watch(&self.watch_dir, RecursiveMode::NonRecursive)?;

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

            Ok(())
        });

        Ok(watch_cancel)
    }

    fn on_file_complete(&self, path: &Path) -> bool {
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
