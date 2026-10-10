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

use std::sync::{Arc, Mutex};

use steve::{BeginProgress, Progress};
use tokio::sync::mpsc::UnboundedSender;

use super::ProgressBar;

type ToMessage<M> = fn(Vec<ProgressBar>) -> M;

pub struct ProgressChannel<M> {
    state: Arc<ProgressState<M>>,
}

impl<M> ProgressChannel<M> {
    pub fn new(channel: UnboundedSender<M>, to_message: ToMessage<M>) -> Self {
        let state = ProgressState::new(channel, to_message);
        Self { state: Arc::new(state) }
    }
}

impl<M: Send + 'static> BeginProgress for ProgressChannel<M> {
    type Handle = ProgressHandle<M>;

    fn begin<S: Into<String>>(&self, message: S, length: usize) -> Self::Handle {
        let bar = ProgressBar {
            label: message.into(),
            current: 0,
            length: length,
        };

        ProgressHandle {
            index: self.state.add(bar),
            state: self.state.clone(),
        }
    }
}

struct ProgressState<M> {
    sender: UnboundedSender<M>,
    to_message: ToMessage<M>,
    bars: Mutex<Vec<ProgressBar>>,
}

impl<M> ProgressState<M> {
    fn new(channel: UnboundedSender<M>, to_message: ToMessage<M>) -> Self {
        Self {
            sender: channel,
            to_message,
            bars: Mutex::new(vec![]),
        }
    }

    fn publish(&self, progress: &Vec<ProgressBar>) {
        let snapshot = progress.clone();
        self.sender.send((self.to_message)(snapshot)).unwrap();
    }

    fn add(&self, bar: ProgressBar) -> usize {
        let mut bars = self.bars.lock().unwrap();
        bars.push(bar);

        if bars.len() > 2 {
            panic!("My dodgy impl assumes max 2 progress bars");
        }

        self.publish(&bars);

        bars.len() - 1
    }

    fn update(&self, index: usize, f: impl FnOnce(&mut ProgressBar)) {
        let mut bars = self.bars.lock().unwrap();
        if let Some(bar) = bars.get_mut(index) {
            f(bar);

            self.publish(&bars);
        }
    }

    fn remove(&self, index: usize) {
        let mut bars = self.bars.lock().unwrap();
        bars.remove(index);
        self.publish(&bars);
    }
}

pub struct ProgressHandle<M> {
    index: usize,
    state: Arc<ProgressState<M>>,
}

impl<M> Progress for ProgressHandle<M> {
    fn set_length(&self, len: usize) {
        self.state.update(self.index, |p| p.length = len);
    }

    fn set_position(&self, pos: usize) {
        self.state.update(self.index, |p| p.current = pos);
    }
}

impl<M> Drop for ProgressHandle<M> {
    fn drop(&mut self) {
        self.state.remove(self.index);
    }
}
