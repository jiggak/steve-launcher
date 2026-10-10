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

use std::{path::{Path, PathBuf}, rc::Rc};

use anyhow::{Result, anyhow};
use ratatui::Frame;
use steve::{FileDownload, InstallTarget, Installer, Modpack, ModpackId, ModpackVersionManifest};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use super::{Action, Component, DownloadBlocked, Event, ProgressBar, ProgressBars, ProgressChannel};

enum InstallState {
    InstallPack {
        component: ProgressBars,
        receiver: UnboundedReceiver<InstallerMessage>,
    },
    DownloadBlocked(DownloadBlocked),
}

pub struct InstallPack<T> {
    instance: Rc<T>,
    pack: ModpackVersionManifest,
    state: InstallState,
}

impl<T: InstallTarget> InstallPack<T> {
    pub fn start_install(instance: Rc<T>, pack: ModpackVersionManifest) -> Self {
        let receiver = start_installer_thread(&instance.install_dir(), &pack);

        Self {
            instance,
            pack,
            state: InstallState::InstallPack {
                component: ProgressBars::new(),
                receiver,
            }
        }
    }

    fn finish_install(&mut self, pack_files: &Vec<PathBuf>) -> Result<()> {
        let instance = Rc::get_mut(&mut self.instance)
            .ok_or(anyhow!("Unable to get mutable instance"))?;

        let installer = Installer::new(&instance.install_dir());

        instance.set_mc_version(self.pack.get_minecraft_version()?)?;
        instance.set_mod_loader(self.pack.get_mod_loader()?)?;

        if let Some(modpack) = &instance.get_modpack_manifest() {
            let old_files = modpack.files_to_paths();

            installer.clean_pack_files(&old_files, &pack_files)?;
        }

        let pack_files: Vec<_> = pack_files.iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect();

        instance.set_modpack_manifest(
            Modpack {
                id: ModpackId::CurseForge {
                    mod_id: self.pack.pack_id,
                    version: self.pack.name.to_string()
                },
                files: pack_files
            }
        )?;

        Ok(())
    }
}

impl<T: InstallTarget> Component for InstallPack<T> {
    fn draw(&self, frame: &mut Frame) {
        match &self.state {
            InstallState::InstallPack { component, .. } => {
                component.draw(frame);
            },
            InstallState::DownloadBlocked(component) => {
                component.draw(frame);
            }
        }
    }

    async fn handle_event(&mut self, event: &Event) -> anyhow::Result<Action> {
        let action = match &mut self.state {
            InstallState::InstallPack { component, receiver } => {
                while let Ok(event) = receiver.try_recv() {
                    match event {
                        InstallerMessage::Progress(progress) => {
                            component.set_progress(progress);
                        },
                        InstallerMessage::Complete { pack_files, blocked_downloads } => {
                            if let Some(blocked_downloads) = blocked_downloads {
                                let component = DownloadBlocked::new(blocked_downloads);
                                self.state = InstallState::DownloadBlocked(component);
                                return Ok(Action::Noop);
                            } else {
                                // FIXME On this call to handle_event we processed several `Progress`
                                // events, then `Complete` was sent causing the UI to skip rendering
                                // that last few percent of the progress (less than ideal).
                                self.finish_install(&pack_files)?;
                                return Ok(Action::Quit);
                            }
                        },
                        InstallerMessage::Error(e) => {
                            return Err(e);
                        }
                    }
                }

                Action::Noop
            },
            InstallState::DownloadBlocked(component) => {
                component.handle_event(event).await?
            }
        };

        Ok(action)
    }
}

enum InstallerMessage {
    Complete {
        pack_files: Vec<PathBuf>,
        blocked_downloads: Option<Vec<FileDownload>>,
    },
    Progress(Vec<ProgressBar>),
    Error(anyhow::Error),
}

fn start_installer_thread(
    install_dir: &Path,
    pack: &ModpackVersionManifest
) -> UnboundedReceiver<InstallerMessage> {
    let (tx, rx) = mpsc::unbounded_channel();

    let install_dir = install_dir.to_path_buf();
    let pack = pack.clone();

    tokio::spawn(async move {
        let progress = ProgressChannel::new(tx.clone(), InstallerMessage::Progress);

        let installer = Installer::new(&install_dir);
        let result = installer.install_pack(&pack, false, &progress).await;
        match result {
            Ok((pack_files, blocked_downloads)) => {
                tx.send(InstallerMessage::Complete { pack_files, blocked_downloads }).unwrap();
            },
            Err(e) => {
                tx.send(InstallerMessage::Error(e)).unwrap();
            }
        }
    });

    rx
}
