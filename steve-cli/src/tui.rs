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
    type Output;

    fn draw(&mut self, frame: &mut Frame, area: Rect);

    async fn handle_event(&mut self, event: &Event) -> Result<Action<Self::Output>>;
}

#[derive(Debug)]
pub enum Action<T> {
    /// No action; app continues run loop
    Continue,
    /// Quit app
    Quit,
    /// Component produced result
    Done(T),
}

pub async fn run<C: Component>(component: &mut C) -> Result<Option<C::Output>> {
    let mut events = EventStreamThread::start(20.0);
    let mut terminal = ratatui::init();

    let result = async {
        loop {
            terminal.draw(|f| component.draw(f, f.area()))?;

            let event = events.next().await?;
            match component.handle_event(&event).await? {
                Action::Quit => return Ok(None),
                Action::Done(output) => return Ok(Some(output)),
                Action::Continue => { },
            }
        }
    }.await;

    // restore even if the loop errored
    events.stop();
    ratatui::restore();

    result
}

pub enum ComponentChain<A, B, F> {
    First { a: A, f: Option<F> },
    Next(B),
}

impl<A, B, F> Component for ComponentChain<A, B, F>
where
    A: Component,
    B: Component,
    F: AsyncFnOnce(A::Output) -> Result<B>,
{
    type Output = B::Output;

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        match self {
            ComponentChain::First { a, .. } => a.draw(frame, area),
            ComponentChain::Next(b) => b.draw(frame, area),
        }
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Action<B::Output>> {
        match self {
            ComponentChain::First { a, f } => match a.handle_event(event).await? {
                Action::Continue => Ok(Action::Continue),
                Action::Quit => Ok(Action::Quit),
                Action::Done(output) => {
                    let f = f.take().unwrap();
                    *self = ComponentChain::Next(f(output).await?);
                    Ok(Action::Continue)
                },
            },
            ComponentChain::Next(b) => b.handle_event(event).await,
        }
    }
}

pub trait ComponentFlow: Component + Sized {
    fn then<B, F>(self, f: F) -> ComponentChain<Self, B, F>
        where B: Component, F: AsyncFnOnce(Self::Output) -> Result<B>,
    {
        ComponentChain::First { a: self, f: Some(f) }
    }
}

impl<C: Component> ComponentFlow for C { }
