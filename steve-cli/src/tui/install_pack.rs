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
use steve::{
    CurseForgeZip, FileDownload, InstallTarget, Installer, Modpack, ModpackId,
    ModpackVersionManifest
};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use super::{
    Action, Component, DownloadBlocked, Event, ProgressBar, ProgressBars,
    ProgressChannel
};

enum State {
    InstallPack {
        component: ProgressBars,
        receiver: UnboundedReceiver<InstallerMessage>,
    },
    DownloadBlocked {
        component: DownloadBlocked,
        installed_files: Vec<PathBuf>,
    },
}

pub struct InstallPack<T, P> {
    instance: T,
    pack: P,
    state: State,
}

impl<T: InstallTarget, P: InstallHandler> InstallPack<T, P> {
    pub fn start_install(instance: T, pack: P) -> Self
        where P: InstallHandler
    {
        let receiver = pack.start_install_thread(&instance.install_dir());

        Self {
            instance,
            pack,
            state: State::InstallPack {
                component: ProgressBars::new(),
                receiver,
            }
        }
    }
}

impl<T: InstallTarget, P: InstallHandler> Component for InstallPack<T, P> {
    type Output = ();

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.state {
            State::InstallPack { component, .. } => {
                component.draw(frame, area);
            },
            State::DownloadBlocked { component, .. } => {
                component.draw(frame, area);
            }
        }
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Action<Self::Output>> {
        let action = match &mut self.state {
            State::InstallPack { component, receiver } => {
                while let Ok(event) = receiver.try_recv() {
                    match event {
                        InstallerMessage::Progress(progress) => {
                            component.set_progress(progress);
                        },
                        InstallerMessage::Complete { pack_files, blocked_downloads } => {
                            if let Some(blocked_downloads) = blocked_downloads {
                                let component = DownloadBlocked::start(
                                    self.instance.install_dir(), blocked_downloads
                                )?;

                                if let Some(component) = component {
                                    self.state = State::DownloadBlocked {
                                        component,
                                        installed_files: pack_files
                                    };
                                    return Ok(Action::Continue);
                                } else {
                                    self.pack.finish_install(&mut self.instance, &pack_files)?;
                                    return Ok(Action::Quit);
                                }
                            } else {
                                self.pack.finish_install(&mut self.instance, &pack_files)?;
                                return Ok(Action::Quit);
                            }
                        },
                        InstallerMessage::Error(e) => {
                            return Err(e);
                        }
                    }
                }

                Action::Continue
            },
            State::DownloadBlocked { component, installed_files } => {
                match component.handle_event(event).await? {
                    Action::Done(_) => {
                        self.pack.finish_install(&mut self.instance, installed_files)?;
                        Action::Quit
                    },
                    other => other
                }
            }
        };

        Ok(action)
    }
}

pub enum InstallerMessage {
    Complete {
        pack_files: Vec<PathBuf>,
        blocked_downloads: Option<Vec<FileDownload>>,
    },
    Progress(Vec<ProgressBar>),
    Error(anyhow::Error),
}

pub  trait InstallHandler {
    fn start_install_thread(&self, install_dir: &Path) -> UnboundedReceiver<InstallerMessage>;
    fn finish_install<T>(&self, instance: &mut T, installed_files: &Vec<PathBuf>) -> Result<()>
        where T : InstallTarget;
}

impl InstallHandler for ModpackVersionManifest {
    fn start_install_thread(&self, install_dir: &Path) -> UnboundedReceiver<InstallerMessage> {
        let (tx, rx) = mpsc::unbounded_channel();

        let install_dir = install_dir.to_path_buf();
        let pack = self.clone();

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

    fn finish_install<T>(&self, instance: &mut T, installed_files: &Vec<PathBuf>) -> Result<()>
        where T : InstallTarget
    {
        let installer = Installer::new(&instance.install_dir());

        instance.set_mc_version(self.get_minecraft_version()?)?;
        instance.set_mod_loader(self.get_mod_loader()?)?;

        if let Some(modpack) = &instance.get_modpack_manifest() {
            let old_files = modpack.files_to_paths();

            installer.clean_pack_files(&old_files, &installed_files)?;
        }

        let pack_files: Vec<_> = installed_files.iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect();

        instance.set_modpack_manifest(
            Modpack {
                id: ModpackId::CurseForge {
                    mod_id: self.pack_id,
                    version: self.name.to_string()
                },
                files: pack_files
            }
        )?;

        Ok(())
    }
}

impl InstallHandler for CurseForgeZip {
    fn start_install_thread(&self, install_dir: &Path) -> UnboundedReceiver<InstallerMessage> {
        let (tx, rx) = mpsc::unbounded_channel();

        let install_dir = install_dir.to_path_buf();
        let pack = self.clone();

        tokio::spawn(async move {
            let progress = ProgressChannel::new(tx.clone(), InstallerMessage::Progress);

            let installer = Installer::new(&install_dir);
            let result = installer.install_pack_zip(&pack, &progress).await;
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

    fn finish_install<T>(&self, instance: &mut T, installed_files: &Vec<PathBuf>) -> Result<()>
        where T : InstallTarget
    {
        let installer = Installer::new(&instance.install_dir());

        instance.set_mc_version(self.manifest.minecraft.version.clone())?;
        instance.set_mod_loader(self.manifest.minecraft.get_mod_loader()?)?;

        if let Some(modpack) = &instance.get_modpack_manifest() {
            let old_files = modpack.files_to_paths();

            installer.clean_pack_files(&old_files, &installed_files)?;
        }

        let pack_files: Vec<_> = installed_files.iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect();

        instance.set_modpack_manifest(
            Modpack {
                id: ModpackId::CurseZip {
                    pack_name: self.manifest.name.clone(),
                    version: self.manifest.version.clone(),
                },
                files: pack_files,
             }
        )?;

        Ok(())
    }
}
