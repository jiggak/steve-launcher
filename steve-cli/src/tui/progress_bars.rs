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

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Stylize},
    text::Line,
    widgets::{Block, Borders, Gauge, Padding},
};

use super::{Action, Component, Event};

pub struct ProgressBars {
    bars: Vec<ProgressBar>,
}

impl ProgressBars {
    pub fn new() -> Self {
        Self { bars: vec![] }
    }

    pub fn set_progress(&mut self, bars: Vec<ProgressBar>) {
        self.bars = bars;
    }
}

impl Component for ProgressBars {
    type Output = ();

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::vertical([Constraint::Length(4), Constraint::Length(4)]);
        let areas = area.layout_vec(&layout);

        for (i, bar) in self.bars.iter().enumerate() {
            render_bar(&bar, frame, areas[i]);
        }
    }

    async fn handle_event(&mut self, _event: &Event) -> anyhow::Result<Action<Self::Output>> {
        Ok(Action::Continue)
    }
}

fn render_bar(bar: &ProgressBar, frame: &mut Frame, area: Rect) {
    let title = Line::from(bar.label.as_str()).centered();
    let title = Block::new()
        .borders(Borders::NONE)
        .padding(Padding::vertical(1))
        .title(title)
        .fg(Color::Blue);

    let widget = Gauge::default()
        .block(title)
        .gauge_style(Color::Blue)
        .percent(bar.percentage());

    frame.render_widget(widget, area);
}

#[derive(Clone)]
pub struct ProgressBar {
    pub label: String,
    pub current: usize,
    pub length: usize,
}

impl ProgressBar {
    fn percentage(&self) -> u16 {
        ((100 * self.current) / self.length) as u16
    }
}
