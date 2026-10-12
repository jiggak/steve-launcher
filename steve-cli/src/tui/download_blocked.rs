/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2026 Josh Kropf <josh@slashdev.ca>
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

use std::{io::Result as IoResult, path::Path, process::{Command, Stdio}, sync::{Arc, Mutex}};

use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{layout::{Constraint, Layout}, style::Stylize, text::Line, widgets::{Block, Paragraph}};
use steve::{FileDownload, Installer, WatchList, WatcherHandle, watch_downloads};

use crate::tui::Component;
use super::{Action, Event};

pub struct DownloadBlocked {
    blocked_downloads: Vec<FileDownload>,
    watch_list: Arc<Mutex<WatchList>>,
    _watcher: WatcherHandle,
}

impl DownloadBlocked {
    pub fn start<P: AsRef<Path>>(
        dest_dir: P,
        blocked_downloads: Vec<FileDownload>
    ) -> Result<Self> {
        let installer = Installer::new(dest_dir.as_ref());

        let watch_list = WatchList::new(
            blocked_downloads.iter()
                .map(|f| f.file_name.as_str())
        );

        // copy any downloads already in watch dir
        for f in &blocked_downloads {
            if watch_list.is_file_complete(&f.file_name) {
                let file_path = watch_list.watch_dir.join(&f.file_name);
                installer.install_file(f, &file_path)?;
            }
        }

        let watch_list = Arc::new(Mutex::new(watch_list));

        let _watcher = start_download_watcher(
            watch_list.clone(),
            blocked_downloads.clone(),
            installer
        )?;

        Ok(Self {
            blocked_downloads,
            watch_list,
            _watcher,
        })
    }

    fn open_pending_downloads(&self) -> IoResult<()> {
        let watch_list = self.watch_list.lock().unwrap();
        let pending_downloads = self.blocked_downloads.iter()
            .filter_map(|d| match watch_list.is_file_complete(&d.file_name) {
                false => Some(d.url.as_str()),
                _ => None
            });

        open_urls(pending_downloads)
    }
}

impl Component for DownloadBlocked {
    fn draw(&self, frame: &mut ratatui::Frame) {
        // "Files below must be downloaded manually. Press [o] to open all, [q] to quit."

        let layout = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
        let [title_area, body_area] = frame.area()
            .layout(&layout);

        let title = Line::from("Files below must be downloaded manually")
            .centered()
            .bold();

        let mut lines = vec![];

        let watch_list = self.watch_list.lock().unwrap();
        for d in &self.blocked_downloads {
            let status = match watch_list.is_file_complete(&d.file_name) {
                true => "✅",
                false => "❌"
            };

            lines.push(Line::from(format!("{status} {}", d.url)))
        }

        let block = Block::bordered()
            .title_bottom("(o) open all links | (q) quit");

        let list = Paragraph::new(lines)
            .block(block);

        frame.render_widget(title, title_area);
        frame.render_widget(list, body_area);
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Action> {
        if self.watch_list.lock().unwrap().is_all_complete() {
            return Ok(Action::Complete);
        }

        match event {
            Event::Key(KeyCode::Char('o')) => {
                self.open_pending_downloads()?
            },
            Event::Key(KeyCode::Char('q')) => {
                return Ok(Action::Quit);
            },
            _ => { }
        }

        Ok(Action::Noop)
    }
}

fn start_download_watcher(
    watch_list: Arc<Mutex<WatchList>>,
    blocked_downloads: Vec<FileDownload>,
    installer: Installer,
) -> Result<WatcherHandle> {
    let watcher = watch_downloads(move |file_path| {
        let mut watch_list = watch_list.lock().unwrap();
        if watch_list.on_file_complete(&file_path) {
            let file_name = file_path.file_name().unwrap().to_string_lossy();
            let file = blocked_downloads.iter()
                .find(|d| d.file_name == file_name)
                .unwrap();
            installer.install_file(file, &file_path)
                .expect("Downlaod watcher should install file successfully");
        }
    })?;

    Ok(watcher)
}

fn open_urls<'a, T>(urls: T) -> IoResult<()>
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
