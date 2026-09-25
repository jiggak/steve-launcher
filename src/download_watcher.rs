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

use notify_debouncer_full::{
    DebounceEventResult, Debouncer, NoCache, new_debouncer,
    notify::{INotifyWatcher, RecursiveMode, Result}
};
use std::{
    collections::HashMap, path::{Path, PathBuf}, sync::mpsc::Sender,
    time::Duration
};

use crate::env;

pub enum WatcherMessage {
    AllComplete,
    FileComplete(PathBuf),
    // FIXME it's kinda weird having this variant here instead of cli app
    KeyPress(char)
}

pub struct WatcherHandle {
    _debouncer: Debouncer<INotifyWatcher, NoCache>
}

impl WatcherHandle {
    pub fn stop(self) {
        drop(self)
    }
}

pub struct WatchList {
    watch_list: HashMap<String, bool>,
    pub watch_dir: PathBuf
}

impl WatchList {
    pub fn new<I, S>(file_names: I) -> WatchList
        where I: Iterator<Item = S>, S: AsRef<str>
    {
        let watch_dir = env::get_downloads_dir();
        let watch_list = file_names
            .map(|f| (f.as_ref().to_string(), watch_dir.join(f.as_ref()).exists()))
            .collect();

        Self { watch_list, watch_dir }
    }

    pub fn on_file_complete(&mut self, path: &Path) -> bool {
        let path_file_name = path.file_name()
            .and_then(|p| p.to_str())
            .unwrap();

        if let Some(value) = self.watch_list.get_mut(path_file_name) {
            *value = true;
            true
        } else {
            false
        }
    }

    pub fn is_file_complete(&self, file_name: &String) -> bool {
        match self.watch_list.get(file_name) {
            Some(v) => *v,
            None => false
        }
    }

    pub fn is_all_complete(&self) -> bool {
        self.watch_list.values().all(|v| *v)
    }
}

pub fn watch_downloads(tx: Sender<WatcherMessage>) -> Result<WatcherHandle> {
    let watch_dir = env::get_downloads_dir();

    let mut debouncer = new_debouncer(Duration::from_secs(1), None, move |result: DebounceEventResult| {
        match result {
            Ok(events) => {
                for event in events {
                    for path in &event.paths {
                        if is_download_complete(path) {
                            tx.send(WatcherMessage::FileComplete(path.clone())).unwrap();
                        }
                    }
                }
            },
            Err(_errors) => { },
        }
    })?;

    debouncer.watch(watch_dir, RecursiveMode::NonRecursive)?;

    Ok(WatcherHandle {
        _debouncer: debouncer
    })
}

fn is_download_complete(path: &Path) -> bool {
    if !path.is_file() {
        false
    } else {
        match path.extension().and_then(|e| e.to_str()) {
            Some("crdownload") | Some("part") | Some("download") => false,
            _ => true,
        }
    }
}
