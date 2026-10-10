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

use anyhow::{Result, anyhow};
use crossterm::event::{Event as TermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind};
use futures::{FutureExt, StreamExt};
use tokio::{sync::mpsc, task::JoinHandle};

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Error,
    Tick,
    Key(KeyCode),
}

pub struct EventStreamThread {
    rx: mpsc::UnboundedReceiver<Event>,
    task: Option<JoinHandle<()>>,
}

impl EventStreamThread {
    pub fn start(ticks_per_s: f32) -> Self {
        let tick_delay = std::time::Duration::from_secs_f32(1.0 / ticks_per_s);

        let (tx, rx) = mpsc::unbounded_channel();

        let task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_delay);

            loop {
                let tick_delay = tick_interval.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    maybe_event = crossterm_event => {
                        match maybe_event {
                            Some(Ok(TermEvent::Key(KeyEvent { code, kind: KeyEventKind::Press, .. }))) => {
                                tx.send(Event::Key(code)).unwrap();
                            }
                            Some(Err(_)) => {
                                tx.send(Event::Error).unwrap();
                            }
                            _ => {},
                        }
                    },
                    _ = tick_delay => {
                        tx.send(Event::Tick).unwrap();
                    },
                }
            }
        });

        Self {
            rx,
            task: Some(task),
        }
    }

    pub async fn next(&mut self) -> Result<Event> {
        self.rx.recv().await
            .ok_or(anyhow!("Unable to get event"))
    }

    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
