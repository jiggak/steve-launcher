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

mod confirm_prompt;
mod debounce;
mod download_blocked;
mod event_stream;
mod install_pack;
mod modpack_search;
mod modpack_update;
mod modpack_zip_install;
mod progress_bars;
mod progress_channel;
mod select_list;
mod text_input;

pub use {
    confirm_prompt::ConfirmPrompt,
    download_blocked::DownloadBlocked,
    event_stream::{Event, EventStreamThread},
    install_pack::InstallPack,
    modpack_search::ModpackSearch,
    modpack_update::ModpackUpdate,
    modpack_zip_install::ModpackZipInstall,
    progress_bars::{ProgressBar, ProgressBars},
    progress_channel::ProgressChannel,
    select_list::{ListEntry, SelectList},
    text_input:: TextInput,
};

use anyhow::Result;
use ratatui::{Frame, layout::Rect};

pub trait Component {
    fn draw(&mut self, frame: &mut Frame, area: Rect);

    async fn handle_event(&mut self, event: &Event) -> Result<Action>;
}

#[derive(Debug)]
pub enum Action {
    /// No action (probably should use Option<Action> instead)
    Noop,
    /// Quit app
    Quit,
    /// User confirmed prompt
    Confirm(bool),
    /// Component completed
    Complete,
    /// User changed text input
    Change(String),
    /// User selected item in list
    Select(usize),
}

pub struct App<C> {
    component: C,
}

impl<C: Component> App<C> {
    pub fn new(component: C) -> Self {
        Self {
            component,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut events = EventStreamThread::start(20.0);
        let mut terminal = ratatui::init();

        loop {
            let event = events.next().await?;
            let action = self.handle_event(&event).await?;

            if let Action::Quit = action {
                break;
            }

            terminal.draw(|frame| {
                self.draw(frame);
            })?;
        }

        events.stop();
        ratatui::restore();

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        self.component.draw(frame, frame.area());
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Action> {
        self.component.handle_event(event).await
    }
}
