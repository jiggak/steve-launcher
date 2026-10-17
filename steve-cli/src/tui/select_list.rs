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
use ratatui::{
    Frame, layout::Rect, style::Style, text::Text, widgets::{List, ListState}
};

use crate::tui::{Action, Component, Event};

pub trait ListEntry {
    fn to_text(&self, width: usize) -> Text<'_>;
}

pub struct SelectList<T: ListEntry> {
    items: Vec<T>,
    list_state: ListState,
}

impl<T: ListEntry> SelectList<T> {
    pub fn new() -> Self {
        Self {
            items: vec![],
            list_state: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn items(&self) -> &Vec<T> {
        &self.items
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
    }
}

impl<T: ListEntry> Component for SelectList<T> {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let list_items = self.items.iter()
            .map(|s| s.to_text(area.width as usize - 2));

        let list = List::new(list_items)
            .highlight_style(Style::new().reversed())
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Action> {
        match event {
            Event::Key(KeyCode::Up) => self.list_state.select_previous(),
            Event::Key(KeyCode::Down) => self.list_state.select_next(),
            Event::Key(KeyCode::Enter) => {
                if let Some(index) = self.list_state.selected() {
                    return Ok(Action::Select(index));
                }
            },
            _ => { }
        }

        Ok(Action::Noop)
   }
}
