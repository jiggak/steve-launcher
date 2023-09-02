use serde::Deserialize;

use super::empty_string_is_none;
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
    pub release_type: String
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
    pub id: u32,
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
            .ok_or(Error::new("Missing 'minecraft' target in modpack manifest"))
    }

    pub fn get_forge_version(&self) -> Option<String> {
        self.targets.iter()
            .find(|t| t.name == "forge")
            .map(|t| t.version.clone())
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
    pub project_id: u64,
    #[serde(rename(deserialize = "file"))]
    pub file_id: u64
}
