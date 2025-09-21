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
    json::{CurseForgeFile, CurseForgeMod, ModpackVersionManifest},
    AssetClient, CurseForgeZip, Error, Progress
};

pub struct Installer {
    dest_dir: PathBuf,
    client: AssetClient
}

impl Installer {
    pub fn new(dest_dir: &Path) -> Self {
        Self {
            dest_dir: dest_dir.into(),
            client: AssetClient::new()
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

    fn get_file_type_dir(&self, file_type: &FileType) -> PathBuf {
        match file_type {
            FileType::Mod => self.mods_dir(),
            FileType::Resource => self.resource_pack_dir(),
            FileType::Shaders => self.shader_pack_dir()
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
        progress: &dyn Progress
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
        progress: &dyn Progress
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

        progress.begin("Downloading assets...", assets.len());

        for (i, f) in assets.iter().enumerate() {
            progress.advance(i + 1);

            // curse packs from modpacks.ch could include a single asset file
            // which is the full curse zip file, download and extract overrides
            if f.file_type == "cf-extract" {
                let dest_file_path = std::env::temp_dir().join(&f.name);
                self.client.download_file(f.url.as_ref().unwrap(), &dest_file_path)
                    .await?;

                let pack = CurseForgeZip::load_zip(&dest_file_path)?;
                pack.copy_game_data(&self.dest_dir)?;

                let mut override_files = pack.list_overrides()?;
                installed_files.append(&mut override_files);

                continue;
            }

            let dest_file_path = self.dest_dir.join(&f.path).join(&f.name);

            // save time/bandwidth and skip download if dest file exists
            if !dest_file_path.exists() {
                self.client.download_file(f.url.as_ref().unwrap(), &dest_file_path)
                    .await?;
            }

            installed_files.push(dest_file_path);
        }

        progress.end();

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

    async fn download_curseforge_files(&self,
        file_ids: Vec<u64>,
        project_ids: Vec<u64>,
        mut installed_files: Vec<PathBuf>,
        progress: &dyn Progress
    ) -> Result<(Vec<PathBuf>, Option<Vec<FileDownload>>)> {
        let mut file_list = self.client.get_curseforge_file_list(&file_ids).await?;
        let mut mod_list = self.client.get_curseforge_mods(&project_ids).await?;

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

        progress.begin("Downloading mods...", downloads.len());

        // create mods dir in case there are zero automated downloads with one or more manual downloads
        fs::create_dir_all(self.mods_dir())?;

        for (i, f) in downloads.iter().enumerate() {
            progress.advance(i + 1);

            let dest_file_path = self.get_file_path(f);

            // save time/bandwidth and skip download if dest file exists
            if dest_file_path.exists() {
                continue;
            }

            self.client.download_file(&f.url, &dest_file_path).await?;
        }

        progress.end();

        installed_files.extend(
            file_downloads.iter()
                .map(|f| self.get_file_path(f))
        );

        let delete_files = [
            list_files_for_delete(&self.mods_dir(), &installed_files)?,
            list_files_for_delete(&self.resource_pack_dir(), &installed_files)?,
            list_files_for_delete(&self.shader_pack_dir(), &installed_files)?
        ].concat();

        if !blocked.is_empty() {
            Ok((delete_files, Some(blocked)))
        } else {
            Ok((delete_files, None))
        }
    }
}

#[derive(Clone)]
pub enum FileType {
    Mod,
    Resource,
    Shaders
}

#[derive(Clone)]
pub struct FileDownload {
    pub file_name: String,
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
            x => panic!("Unimplemented curseforge class_id {x}")
        };

        // url for user to download the file manually
        let user_dl_url = format!("{site_url}/download/{file_id}",
            site_url = m.links.website_url, file_id = f.file_id);

        FileDownload {
            file_name: f.file_name.clone(),
            file_type,
            can_auto_download: f.download_url.is_some(),
            url: match &f.download_url {
                Some(v) => v.clone(),
                None => user_dl_url
            }
        }
    }
}

fn list_files_for_delete(dir: &Path, keep_files: &Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut delete_files = Vec::new();

    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;

            if !entry.file_type()?.is_file() {
                continue;
            }

            let path = entry.path();
            if !keep_files.contains(&path) {
                delete_files.push(path);
            }
        }
    }

    Ok(delete_files)
}
