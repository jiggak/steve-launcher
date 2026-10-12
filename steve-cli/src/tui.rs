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
mod download_blocked;
mod event_stream;
mod install_pack;
mod modpack_update;
mod progress_bars;
mod progress_channel;

pub use {
    confirm_prompt::ConfirmPrompt,
    download_blocked::DownloadBlocked,
    event_stream::{Event, EventStreamThread},
    install_pack::InstallPack,
    modpack_update::ModpackUpdate,
    progress_bars::{ProgressBar, ProgressBars},
    progress_channel::ProgressChannel,
};

use anyhow::Result;
use ratatui::Frame;

pub trait Component {
    fn draw(&self, frame: &mut Frame);
    async fn handle_event(&mut self, event: &Event) -> Result<Action>;
}

#[derive(Debug)]
pub enum Action {
    /// No action (probably should use Option<Action> instead)
    Noop,
    /// Quit app
    Quit,
    /// User confirmed prompt
    Confirm,
    /// Component completed
    Complete,
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

    fn draw(&self, frame: &mut Frame) {
        self.component.draw(frame);
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Action> {
        self.component.handle_event(event).await
    }
}
