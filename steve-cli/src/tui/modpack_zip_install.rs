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

use std::path::{Path, PathBuf};

use anyhow::Result;
use ratatui::{Frame, layout::Rect};
use steve::{CurseForgeZip, Instance};

use crate::tui::{Action, Component, ConfirmPrompt, Event, InstallPack};

enum State {
    Prompt(ConfirmPrompt),
    Install(InstallPack<Instance, CurseForgeZip>),
}

pub struct ModpackZipInstall {
    state: State,
    pack: CurseForgeZip,
    instance_dir: PathBuf,
}

impl ModpackZipInstall {
    pub async fn with_zip(instance_dir: &Path, pack: CurseForgeZip) -> Result<Self> {
        let state = if Instance::exists(instance_dir) {
            let component = ConfirmPrompt::new(
                "Instance already exists, are you sure you want to install the pack here?"
            );

            State::Prompt(component)
        } else {
            let instance = Instance::create(
                instance_dir,
                &pack.manifest.minecraft.version,
                pack.manifest.minecraft.get_mod_loader()?
            ).await?;

            let component = InstallPack::start_install(
                instance,
                pack.clone()
            );

            State::Install(component)
        };

        Ok(Self {
            state,
            pack,
            instance_dir: instance_dir.to_path_buf(),
        })
    }
}

impl Component for ModpackZipInstall {
    type Output = ();

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.state {
            State::Prompt(component) => component.draw(frame, area),
            State::Install(component) => component.draw(frame, area),
        }
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Action<Self::Output>> {
        let action = match &mut self.state {
            State::Prompt(component) => {
                if let Action::Done(is_yes) = component.handle_event(event).await? {
                    if !is_yes {
                        return Ok(Action::Quit);
                    }

                    let instance = Instance::load(&self.instance_dir)?;

                    let component = InstallPack::start_install(
                        instance,
                        self.pack.clone()
                    );

                    self.state = State::Install(component);
                }

                Action::Continue
            },
            State::Install(component) => {
                component.handle_event(event).await?
            },
        };

        Ok(action)
    }
}
