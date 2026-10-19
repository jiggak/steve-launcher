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

use std::time::Duration;

use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{Frame, layout::{Position, Rect}, text::ToSpan};

use crate::tui::{Action, Component, Event, debounce::Debouncer};

pub struct TextInput {
    debouncer: Debouncer<String>,
    text: String,
}

impl TextInput {
    pub fn new<S: Into<String>>(text: S) -> Self {
        let debouncer = Debouncer::new(Duration::from_millis(500));
        Self {
            debouncer,
            text: text.into(),
        }
    }
}

impl Component for TextInput {
    type Output = String;

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(self.text.to_span(), area);

        frame.set_cursor_position(Position::new(
            area.x + self.text.len() as u16,
            area.y
        ));
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Action<Self::Output>> {
        match event {
            Event::Key(KeyCode::Char(c)) => {
                self.text.push(*c);
                self.debouncer.push(self.text.clone());
            },
            Event::Key(KeyCode::Backspace) => {
                self.text.pop();
                self.debouncer.push(self.text.clone());
            },
            _ => { }
        }

        let action = if let Some(text) = self.debouncer.poll() {
            Action::Done(text)
        } else {
            Action::Continue
        };

        Ok(action)
   }
}
