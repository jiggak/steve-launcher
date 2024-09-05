/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2023 Josh Kropf <josh@slashdev.ca>
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

use serde::Deserialize;

use crate::{Error, ModLoader};

#[derive(Deserialize)]
pub struct CurseForgePack {
    pub minecraft: CurseForgeMinecraft,
    #[serde(rename(deserialize = "manifestType"))]
    pub manifest_type: String,
    #[serde(rename(deserialize = "manifestVersion"))]
    pub manifest_version: u8,
    pub name: String,
    pub version: String,
    pub author: String,
    pub files: Vec<CurseForgePackFile>,
    pub overrides: String
}

impl CurseForgePack {
    pub fn get_file_ids(&self) -> Vec<u64> {
        self.files.iter()
            .map(|f| f.file_id)
            .collect()
    }

    pub fn get_project_ids(&self) -> Vec<u64> {
        self.files.iter()
            .map(|f| f.project_id)
            .collect()
    }
}

#[derive(Deserialize)]
pub struct CurseForgeMinecraft {
    pub version: String,
    #[serde(rename(deserialize = "modLoaders"))]
    pub mod_loaders: Vec<CurseForgeModloader>
}

impl CurseForgeMinecraft {
    pub fn get_mod_loader(&self) -> Result<Option<ModLoader>, Error> {
        let loader_id = self.mod_loaders.iter()
            .find(|l| l.primary)
            .map(|l| l.id.as_str());

        if let Some(loader_id) = loader_id {
            Ok(Some(loader_id.parse()?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Deserialize)]
pub struct CurseForgeModloader {
    pub id: String,
    pub primary: bool
}

#[derive(Deserialize)]
pub struct CurseForgePackFile {
    #[serde(rename(deserialize = "projectID"))]
    pub project_id: u64,
    #[serde(rename(deserialize = "fileID"))]
    pub file_id: u64,
    pub required: bool
}

#[derive(Deserialize)]
pub struct CurseForgeResponse<T> {
    pub data: Vec<T>
}

#[derive(Deserialize)]
// https://docs.curseforge.com/#tocS_File
pub struct CurseForgeFile {
    #[serde(rename(deserialize = "id"))]
    pub file_id: u64,
    #[serde(rename(deserialize = "modId"))]
    pub mod_id: u64,
    #[serde(rename(deserialize = "fileName"))]
    pub file_name: String,
    #[serde(rename(deserialize = "downloadUrl"))]
    pub download_url: Option<String>
}

#[derive(Deserialize)]
// https://docs.curseforge.com/#tocS_Mod
pub struct CurseForgeMod {
    #[serde(rename(deserialize = "id"))]
    pub mod_id: u64,
    pub slug: String,
    pub links: CurseForgeModLinks,
    #[serde(rename(deserialize = "classId"))]
    pub class_id: u64
}

#[derive(Deserialize)]
pub struct CurseForgeModLinks {
    #[serde(rename(deserialize = "websiteUrl"))]
    pub website_url: String,
    #[serde(rename(deserialize = "wikiUrl"))]
    pub wiki_url: Option<String>,
    #[serde(rename(deserialize = "issuesUrl"))]
    pub issues_url: Option<String>,
    #[serde(rename(deserialize = "sourceUrl"))]
    pub source_url: Option<String>
}
