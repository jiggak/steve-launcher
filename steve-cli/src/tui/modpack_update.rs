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
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    text::{Text, ToLine},
    widgets::Paragraph,
};
use steve::{InstallTarget, ModpackVersionManifest, ModpacksClient};

use crate::tui::{Action, Component, ConfirmPrompt, Event, InstallPack};

enum State<T> {
    Confirm {
        component: ConfirmPrompt,
        current_version: String,
        latest_version: String,
    },
    InstallPack(InstallPack<T>),
}

pub struct ModpackUpdate<T> {
    instance: T,
    latest_pack: ModpackVersionManifest,
    state: State<T>,
}

impl<T: InstallTarget> ModpackUpdate<T> {
    pub async fn with_modpack_manifest(
        instance: T,
        pack_id: u32,
        current_version: String,
    ) -> Result<Self> {
        let client = ModpacksClient::new();
        let pack_manifest = client.get_curse_modpack_versions(pack_id).await?;

        // hopefully it's safe to assume first version is latest
        let latest_version = pack_manifest.versions.first()
            .ok_or(anyhow!("Pack manifest has empty `versions` list"))?;

        let latest_pack = client
            .get_curse_modpack(pack_id, latest_version.version_id)
            .await?;

        let latest_version = latest_version.name.clone();

        let confirm_prompt = ConfirmPrompt::new("Would you like to (re)install latest version?");

        let component = Self {
            instance,
            latest_pack,
            state: State::Confirm {
                component: confirm_prompt,
                current_version,
                latest_version,
            },
        };

        Ok(component)
    }
}

impl<T: InstallTarget + Clone> Component for ModpackUpdate<T> {
    fn draw(&self, frame: &mut Frame) {
        match &self.state {
            State::Confirm {
                component,
                current_version,
                latest_version,
            } => {
                let frame_layout = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Fill(1)
                ]);

                let [versions_area, prompt_area] = frame.area().layout(&frame_layout);

                let versions_layout = Layout::horizontal([
                    Constraint::Length(10),
                    Constraint::Fill(1)
                ]).spacing(1);

                let [labels_area, values_area] = versions_area.layout(&versions_layout);

                let labels = Paragraph::new(Text::from(vec![
                    "Current:".to_line(),
                    "Latest:".to_line()
                ])).alignment(Alignment::Right);

                let versions = Paragraph::new(Text::from(vec![
                    current_version.to_line(),
                    latest_version.to_line(),
                ]));

                frame.render_widget(labels, labels_area);
                frame.render_widget(versions, values_area);

                frame.render_widget(component, prompt_area);
            }
            State::InstallPack(install_pack) => {
                install_pack.draw(frame);
            }
        }
    }

    async fn handle_event(&mut self, event: &Event) -> anyhow::Result<Action> {
        let action = match &mut self.state {
            State::Confirm { component: prompt, .. } =>
                prompt.handle_event(event).await?,
            State::InstallPack(install_pack) =>
                install_pack.handle_event(event).await?,
        };

        if let Action::Confirm = action {
            let install_pack = InstallPack::start_install(
                self.instance.clone(),
                self.latest_pack.clone()
            );

            self.state = State::InstallPack(install_pack)
        }

        Ok(action)
    }
}
