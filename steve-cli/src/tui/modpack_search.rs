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
    Frame,
    layout::{Layout, Rect, Spacing},
    style::Stylize,
    symbols::merge::MergeStrategy,
    text::{Line, Span, Text, ToText},
    widgets::{Block, Padding, Paragraph}
};
use steve::{ModpackManifest, ModpackVersionManifest, ModpacksClient};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::tui::{Action, Component, Event, ListEntry, SelectList, TextInput};

pub struct ModpackSearch {
    handle: Option<ThreadHandle>,
    search: TextInput,
    results: SelectList<ModpackManifest>,
    limit: u8,
}

impl ModpackSearch {
    pub fn new(search: &str, limit: u8) -> Self {
        let thread_handle = start_search_thread(search.to_string(), limit);

        Self {
            handle: Some(thread_handle),
            search: TextInput::new(search),
            results: SelectList::new(),
            limit,
        }
    }

    fn handle_thread_events(&mut self) -> Result<Option<ThreadHandle>> {
        if let Some(mut handle) = self.handle.take() {
            while let Ok(event) = handle.receiver.try_recv() {
                match event {
                    ThreadEvent::Results(r) => {
                        self.results.set_items(r);
                        return Ok(None);
                    },
                    ThreadEvent::Status(status) => {
                        handle.status = status;
                    },
                    ThreadEvent::Error(e) => {
                        return Err(e);
                    }
                }
            }

            Ok(Some(handle))
        } else {
            Ok(None)
        }
    }
}

impl Component for ModpackSearch {
    type Output = ModpackVersionManifest;

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        use ratatui::layout::Constraint::*;

        let layout = Layout::vertical(vec![Length(5), Fill(1)])
            .spacing(Spacing::Overlap(1));
        let [search_area, results_area] = area.layout(&layout);

        let search_block = Block::bordered()
            .merge_borders(MergeStrategy::Exact)
            .padding(Padding::uniform(1))
            .title("Search");
        let search_block_inner = search_block.inner(search_area);

        frame.render_widget(search_block, search_area);
        self.search.draw(frame, search_block_inner);

        let results_block = Block::bordered()
            .merge_borders(MergeStrategy::Exact)
            .padding(Padding::uniform(1))
            .title("Results");
        let results_block_inner = results_block.inner(results_area);

        if let Some(handle) = &self.handle {
            let popup_area = results_area.centered(Percentage(60), Length(3));
            let popup = Paragraph::new(handle.status.to_text())
                .block(Block::bordered())
                .centered();
            frame.render_widget(popup, popup_area);
        } else {
            frame.render_widget(results_block, results_area);
            self.results.draw(frame, results_block_inner);
        }
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Action<Self::Output>> {
        self.handle = self.handle_thread_events()?;

        if let Action::Done(search) = self.search.handle_event(event).await? {
            self.handle = Some(start_search_thread(search, self.limit));
        }

        if let Action::Done(index) = self.results.handle_event(event).await? {
            // TODO prompt user to select pack version
            let selected_pack = &self.results.items()[index];
            let selected_version = selected_pack.versions.first().unwrap();

            let client = ModpacksClient::new();

            let pack = if selected_pack.provider == "curseforge" {
                client.get_curse_modpack(selected_pack.pack_id, selected_version.version_id).await?
            } else {
                client.get_ftb_modpack(selected_pack.pack_id, selected_version.version_id).await?
            };

            return Ok(Action::Done(pack));
        }

        let action = match event {
            Event::Key(KeyCode::Esc) => Action::Quit,
            _ => Action::Continue
        };

        Ok(action)
    }
}

struct ThreadHandle {
    receiver: UnboundedReceiver<ThreadEvent>,
    status: String,
}

enum ThreadEvent {
    Results(Vec<ModpackManifest>),
    Status(String),
    Error(anyhow::Error),
}

fn start_search_thread(search: String, limit: u8) -> ThreadHandle {
    let (sender, receiver) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let send_status = |status| sender.send(ThreadEvent::Status(status)).unwrap();
        match search_modpacks(&search, limit, send_status).await {
            Ok(results) => sender.send(ThreadEvent::Results(results)).unwrap(),
            Err(e) => sender.send(ThreadEvent::Error(e)).unwrap(),
        }
    });

    ThreadHandle {
        receiver,
        status: "Searching modpacks...".to_string()
    }
}

async fn search_modpacks(
    search: &String,
    limit: u8,
    progress: impl Fn(String)
) -> Result<Vec<ModpackManifest>> {
    let client = ModpacksClient::new();

    let results = client.search_modpacks(search, limit).await?;

    let mut search_results = Vec::new();

    for pack_id in results.pack_ids {
        progress(format!("Retrieving search result {} of {}", search_results.len() + 1, results.total));
        search_results.push(
            client.get_ftb_modpack_versions(pack_id).await?
        );
    }

    for curse_id in results.curseforge_ids {
        progress(format!("Retrieving search result {} of {}", search_results.len() + 1, results.total));
        search_results.push(
            client.get_curse_modpack_versions(curse_id).await?
        );
    }

    Ok(search_results)
}

impl ListEntry for ModpackManifest {
    fn to_text(&self, width: usize) -> Text<'_> {
        let author = self.authors.first().map_or("", |a| &a.name);

        let left = self.name.as_str().bold();
        let right = format!("by: {}", author).italic().dim();
        let pad = width.saturating_sub(left.width() + right.width());

        Line::from(vec![left, Span::raw(" ".repeat(pad)), right]).into()
    }
}
