/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2024 Josh Kropf <josh@slashdev.ca>
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

use std::{fs, path::{Path, PathBuf}};

use anyhow::{bail, Result};

use crate::{
    AssetClient, BeginProgress, CurseClient, CurseForgeZip, Error, Modpack,
    json::{CurseForgeFile, CurseForgeMod, ModpackVersionManifest}
};

pub struct Installer {
    dest_dir: PathBuf,
    asset_client: AssetClient,
    curse_client: CurseClient
}

pub trait InstallTarget {
    fn install_dir(&self) -> PathBuf;
    fn get_modpack_manifest(&self) -> &Option<Modpack>;
    fn set_modpack_manifest(&mut self, modpack: Modpack) -> Result<()>;
}

impl Installer {
    pub fn new(dest_dir: &Path) -> Self {
        Self {
            dest_dir: dest_dir.into(),
            asset_client: AssetClient::new(),
            curse_client: CurseClient::new()
        }
    }

    fn mods_dir(&self) -> PathBuf {
        self.dest_dir.join("mods")
    }

    fn resource_pack_dir(&self) -> PathBuf {
        self.dest_dir.join("resourcepacks")
    }

    fn shader_pack_dir(&self) -> PathBuf {
        self.dest_dir.join("shaderpacks")
    }

    fn data_pack_dir(&self) -> PathBuf {
        self.dest_dir.join("config/openloader/data")
    }

    fn get_file_type_dir(&self, file_type: &FileType) -> PathBuf {
        match file_type {
            FileType::Mod => self.mods_dir(),
            FileType::Resource => self.resource_pack_dir(),
            FileType::Shaders => self.shader_pack_dir(),
            FileType::Datapack => self.data_pack_dir()
        }
    }

    fn get_file_path(&self, file: &FileDownload) -> PathBuf {
        self.get_file_type_dir(&file.file_type)
            .join(&file.file_name)
    }

    pub fn install_file(&self, file: &FileDownload, src_path: &Path) -> std::io::Result<()> {
        let dest_file = self.get_file_path(file);
        fs::copy(src_path, dest_file)?;
        Ok(())
    }

    pub async fn install_pack_zip(&self,
        pack: &CurseForgeZip,
        progress: &impl BeginProgress
    ) -> Result<(Vec<PathBuf>, Option<Vec<FileDownload>>)> {
        // copy pack overrides to minecraft dir
        pack.copy_game_data(&self.dest_dir)?;
        let installed_files = pack.list_overrides()?;

        let file_ids = pack.manifest.get_file_ids();
        let project_ids = pack.manifest.get_project_ids();

        self.download_curseforge_files(
            file_ids,
            project_ids,
            installed_files,
            progress
        ).await
    }

    pub async fn install_pack(&self,
        pack: &ModpackVersionManifest,
        is_server: bool,
        progress: &impl BeginProgress
    ) -> Result<(Vec<PathBuf>, Option<Vec<FileDownload>>)> {
        let pack_files: Vec<_> = if is_server {
            pack.files.iter()
                .filter(|f| !f.clientonly)
                .collect()
        } else {
            pack.files.iter()
                .collect()
        };

        let assets: Vec<_> = pack_files.iter()
            .filter(|f| f.url.is_some())
            .collect();

        let mut installed_files: Vec<PathBuf> = Vec::new();

        let main_progress = progress.begin("Downloading assets...", assets.len());

        for (i, f) in assets.iter().enumerate() {
            let file_progress = progress.begin(&f.name, f.size as usize);

            let file_url = f.url.as_ref().unwrap();

            // curse packs from modpacks.ch could include a single asset file
            // which is the full curse zip file, download and extract overrides
            if f.file_type == "cf-extract" {
                let dest_file_path = std::env::temp_dir().join(&f.name);

                // override.zip size is often -1
                // use content-length to get accurate file size for progress
                self.asset_client.download_file_with_length(
                    file_url,
                    &dest_file_path,
                    |x| file_progress.set_length(x),
                    |x| file_progress.set_position(x)
                ).await?;

                let pack = CurseForgeZip::load_zip(&dest_file_path)?;
                pack.copy_game_data(&self.dest_dir)?;

                let mut override_files = pack.list_overrides()?;
                installed_files.append(&mut override_files);
            } else {
                let dest_file_path = self.dest_dir.join(&f.path).join(&f.name);

                // save time/bandwidth and skip download if dest file exists
                if !dest_file_path.exists() {
                    self.asset_client.download_file(
                        &file_url,
                        &dest_file_path,
                        |x| file_progress.set_position(x)
                    ).await?;
                }

                installed_files.push(PathBuf::from(&f.path).join(&f.name));
            }

            main_progress.set_position(i + 1);
        }

        main_progress.end();

        let mods: Vec<_> = pack_files.iter()
            .filter_map(|f| f.curseforge.as_ref())
            .collect();

        let file_ids = mods.iter().map(|c| c.file_id).collect();
        let project_ids = mods.iter().map(|c| c.project_id).collect();

        self.download_curseforge_files(
            file_ids,
            project_ids,
            installed_files,
            progress
        ).await
    }

    pub async fn install_curseforge_file(&self,
        mod_id: u32,
        file_id: u32,
        progress: &impl BeginProgress
    ) -> Result<Option<Vec<FileDownload>>> {
        let result = self.download_curseforge_files(
            vec![file_id],
            vec![mod_id],
            vec![],
            progress
        ).await?;

        Ok(result.1)
    }

    async fn download_curseforge_files(&self,
        file_ids: Vec<u32>,
        project_ids: Vec<u32>,
        mut installed_files: Vec<PathBuf>,
        progress: &impl BeginProgress
    ) -> Result<(Vec<PathBuf>, Option<Vec<FileDownload>>)> {
        let mut file_list = self.curse_client.get_files(&file_ids).await?;
        let mut mod_list = self.curse_client.get_mods(&project_ids).await?;

        if file_list.len() != mod_list.len() {
            bail!(Error::CurseFileListMismatch {
                file_list_len: file_list.len(),
                mod_list_len: mod_list.len()
            });
        }

        // sort the lists so that we can zip them into list of pairs
        file_list.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
        mod_list.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));

        let file_downloads: Vec<_> = file_list.iter()
            .zip(mod_list)
            .map(|(f, m)| FileDownload::new(f, &m))
            .collect();

        // filter files that can be auto-downloaded, and those that must be manually downloaded
        let (downloads, blocked): (Vec<_>, Vec<_>) = file_downloads.clone().into_iter()
            .partition(|f| f.can_auto_download);

        let main_progress = progress.begin("Downloading mods...", downloads.len());

        // create mods dir in case there are zero automated downloads with one or more manual downloads
        fs::create_dir_all(self.mods_dir())?;

        for (i, f) in downloads.iter().enumerate() {
            let file_progress = progress.begin(&f.file_name, f.file_size as usize);

            let dest_file_path = self.get_file_path(f);

            // save time/bandwidth and skip download if dest file exists
            if dest_file_path.exists() {
                continue;
            }

            self.asset_client.download_file(
                &f.url,
                &dest_file_path,
                |x| file_progress.set_position(x)
            ).await?;

            main_progress.set_position(i + 1);
        }

        installed_files.extend(
            file_downloads.iter()
                .map(|f| self.get_file_path(f).strip_prefix(&self.dest_dir).unwrap().to_path_buf())
        );

        if !blocked.is_empty() {
            Ok((installed_files, Some(blocked)))
        } else {
            Ok((installed_files, None))
        }
    }

    pub fn clean_pack_files(&self, old_files: &Vec<PathBuf>, new_files: &Vec<PathBuf>) -> Result<()> {
        Ok(crate::fs::remove_diff_files(&self.dest_dir, &old_files, &new_files)?)
    }
}

#[derive(Clone)]
pub enum FileType {
    Mod,
    Resource,
    Shaders,
    Datapack
}

#[derive(Clone)]
pub struct FileDownload {
    pub file_name: String,
    pub file_size: u64,
    pub file_type: FileType,
    pub can_auto_download: bool,
    pub url: String
}

impl FileDownload {
    pub fn new(f: &CurseForgeFile, m: &CurseForgeMod) -> Self {
        // it feels brittle using hard coded classId, but I don't see anything
        // else that can differentiate mods|resource pack|etc
        let file_type = match m.class_id {
            6 => FileType::Mod,
            12 => FileType::Resource,
            6552 => FileType::Shaders,
            6945 => FileType::Datapack,
            x => panic!("Unimplemented curseforge class_id {x}")
        };

        // url for user to download the file manually
        let user_dl_url = format!("{site_url}/download/{file_id}",
            site_url = m.links.website_url, file_id = f.file_id);

        FileDownload {
            file_name: f.file_name.clone(),
            file_size: f.file_size,
            file_type,
            can_auto_download: f.download_url.is_some(),
            url: match &f.download_url {
                Some(v) => v.clone(),
                None => user_dl_url
            }
        }
    }
}
