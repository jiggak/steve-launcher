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

use super::{empty_string_is_none, ModLoader};
use crate::Error;

// https://api.modpacks.ch/public/modpack/all
#[derive(Deserialize)]
pub struct ModpackIndex {
    #[serde(rename(deserialize = "packs"))]
    pub pack_ids: Vec<u32>,
    pub total: u32,
    pub refreshed: u64,
    pub status: String
}

// https://api.modpacks.ch/public/modpack/search/{limit}?term={search term}
#[derive(Deserialize)]
pub struct ModpackSearch {
    #[serde(rename(deserialize = "packs"))]
    pub pack_ids: Vec<u32>,
    #[serde(rename(deserialize = "curseforge"))]
    pub curseforge_ids: Vec<u32>,
    pub total: u32,
    pub limit: u32,
    pub refreshed: u64
}

// https://api.modpacks.ch/public/modpack/{pack_id}
#[derive(Deserialize)]
pub struct ModpackManifest {
    #[serde(rename(deserialize = "id"))]
    pub pack_id: u32,
    pub name: String,
    pub synopsis: String,
    pub description: String,
    pub authors: Vec<ModpackAuthor>,
    pub versions: Vec<ModpackVersion>,
    #[serde(rename(deserialize = "type"))]
    pub release_type: String,
    pub provider: String
}

#[derive(Deserialize)]
pub struct ModpackAuthor {
    pub id: i32,
    pub website: Option<String>,
    pub name: String,
    #[serde(rename(deserialize = "type"))]
    pub author_type: String,
    pub updated: u64
}

#[derive(Deserialize)]
pub struct ModpackVersion {
    #[serde(rename(deserialize = "id"))]
    pub version_id: u32,
    pub name: String,
    #[serde(rename(deserialize = "type"))]
    pub release_type: String,
    pub updated: u64,
    pub private: Option<bool>,
    pub specs: Option<ModpackVersionSpecs>,
    pub targets: Vec<ModpackVersionTarget>
}

#[derive(Deserialize)]
pub struct ModpackVersionSpecs {
    pub id: i32,
    pub minimum: u32,
    pub recommended: u32
}

#[derive(Deserialize)]
pub struct ModpackVersionTarget {
    pub id: i32,
    pub version: String,
    pub name: String,
    #[serde(rename(deserialize = "type"))]
    pub target_type: String,
    pub updated: u64
}

// https://api.modpacks.ch/public/modpack/{pack_id}/{version_id}
#[derive(Deserialize)]
pub struct ModpackVersionManifest {
    #[serde(rename(deserialize = "id"))]
    pub version_id: u32,
    #[serde(rename(deserialize = "parent"))]
    pub pack_id: u32,
    pub name: String,
    pub files: Vec<ModpackFile>,
    pub specs: Option<ModpackVersionSpecs>,
    pub targets: Vec<ModpackVersionTarget>,
    #[serde(rename(deserialize = "type"))]
    pub release_type: String
}

impl ModpackVersionManifest {
    pub fn get_minecraft_version(&self) -> Result<String, Error> {
        self.targets.iter()
            .find(|t| t.name == "minecraft")
            .map(|t| t.version.clone())
            .ok_or(Error::MinecraftTargetNotFound)
    }

    pub fn get_mod_loader(&self) -> Result<Option<ModLoader>, Error> {
        let mod_loader = self.targets.iter()
            .find(|t| t.target_type == "modloader");

        if let Some(mod_loader) = mod_loader {
            Ok(Some(ModLoader {
                name: mod_loader.name.parse()?,
                version: mod_loader.version.clone()
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Deserialize)]
pub struct ModpackFile {
    pub id: u32,
    pub name: String,
    #[serde(rename(deserialize = "type"))]
    pub file_type: String,
    // `version` field could be either a number or string in json response
    // not using this field right now so removing ignoring it
    // #[serde(deserialize_with = "int_to_string")]
    // pub version: String,
    pub path: String,
    #[serde(deserialize_with = "empty_string_is_none")]
    pub url: Option<String>,
    pub sha1: String,
    pub size: i32,
    pub clientonly: bool,
    pub serveronly: bool,
    pub optional: bool,
    pub updated: u64,
    pub curseforge: Option<ModpackFileCurseforge>
}

#[derive(Deserialize)]
pub struct ModpackFileCurseforge {
    #[serde(rename(deserialize = "project"))]
    pub project_id: u32,
    #[serde(rename(deserialize = "file"))]
    pub file_id: u32
}
