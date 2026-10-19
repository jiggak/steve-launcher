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

use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{Frame, layout::Rect, style::Stylize, text::Line};

use crate::tui::{Action, Component, Event};

pub struct ConfirmPrompt {
    prompt: String,
    confirmed: bool,
}

impl ConfirmPrompt {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            confirmed: false,
        }
    }
}

impl Component for ConfirmPrompt {
    type Output = bool;

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let line = Line::from(vec![
            self.prompt.as_str().bold(),
            " (y/n) ".dark_gray(),
            (if self.confirmed { "yes" } else { "no" }).green(),
        ]);

        frame.render_widget(line, area);
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Action<Self::Output>> {
        match event {
            Event::Key(KeyCode::Char('y')) => self.confirmed = true,
            Event::Key(KeyCode::Char('n')) => self.confirmed = false,
            Event::Key(KeyCode::Enter) => {
                return Ok(Action::Done(self.confirmed));
            },
            _ => {}
        }

        Ok(Action::Continue)
    }
}
