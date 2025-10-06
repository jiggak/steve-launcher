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
use serde_repr::{Deserialize_repr, Serialize_repr};

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
    pub fn get_file_ids(&self) -> Vec<u32> {
        self.files.iter()
            .map(|f| f.file_id)
            .collect()
    }

    pub fn get_project_ids(&self) -> Vec<u32> {
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
    pub project_id: u32,
    #[serde(rename(deserialize = "fileID"))]
    pub file_id: u32,
    pub required: bool
}

#[derive(Deserialize)]
pub struct CurseForgeResponse<T> {
    pub data: T
}

#[derive(Deserialize)]
pub struct CurseForgeResponseWithPaging<T> {
    pub data: Vec<T>,
    pub pagination: Pagination
}

#[derive(Deserialize)]
pub struct Pagination {
    pub index: u32,
    #[serde(rename(deserialize = "pageSize"))]
    pub page_size: u32,
    #[serde(rename(deserialize = "resultCount"))]
    pub result_count: u32,
    #[serde(rename(deserialize = "totalCount"))]
    pub total_count: u64
}

#[derive(Deserialize)]
// https://docs.curseforge.com/#tocS_File
pub struct CurseForgeFile {
    #[serde(rename(deserialize = "id"))]
    pub file_id: u32,
    #[serde(rename(deserialize = "modId"))]
    pub mod_id: u32,
    #[serde(rename(deserialize = "isAvailable"))]
    pub is_available: bool,
    #[serde(rename(deserialize = "displayName"))]
    pub display_name: String,
    #[serde(rename(deserialize = "fileName"))]
    pub file_name: String,
    #[serde(rename(deserialize = "releaseType"))]
    pub release_type: FileReleaseType,
    #[serde(rename(deserialize = "fileStatus"))]
    pub file_status: FileStatus,
    pub hashes: Vec<FileHash>,
    #[serde(rename(deserialize = "fileLength"))]
    pub file_size: u64,
    #[serde(rename(deserialize = "downloadUrl"))]
    pub download_url: Option<String>,
    pub dependencies: Vec<FileDependency>,
    #[serde(rename(deserialize = "fileFingerprint"))]
    pub file_fingerprint: u32
}

#[derive(Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum FileReleaseType {
    Release = 1,
    Beta = 2,
    Alpha = 3
}

#[derive(Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum FileStatus {
    Processing = 1,
    ChangesRequired = 2,
    UnderReview = 3,
    Approved = 4,
    Rejected = 5,
    MalwareDetected = 6,
    Deleted = 7,
    Archived = 8,
    Testing = 9,
    Released = 10,
    ReadyForReview = 11,
    Deprecated = 12,
    Baking = 13,
    AwaitingPublishing = 14,
    FailedPublishing = 15,
    Cooking = 16,
    Cooked = 17,
    UnderManualReview = 18,
    ScanningForMalware = 19,
    ProcessingFile = 20,
    PendingRelease = 21,
    ReadyForCooking = 22,
    PostProcessing = 23
}

#[derive(Deserialize)]
pub struct FileHash {
    pub value: String,
    pub algo: HashAlgo
}

#[derive(Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum HashAlgo {
    Sha1 = 1,
    Md5 = 2
}

#[derive(Deserialize)]
pub struct FileDependency {
    #[serde(rename(deserialize = "modId"))]
    pub mod_id: u32,
    #[serde(rename(deserialize = "relationType"))]
    pub algo: FileRelationType
}

#[derive(Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum FileRelationType {
    EmbeddedLibrary = 1,
    OptionalDependency = 2,
    RequiredDependency = 3,
    Tool = 4,
    Incompatible = 5,
    Include = 6
}

#[derive(Serialize_repr, PartialEq)]
#[repr(u8)]
pub enum ModLoaderType {
    Any = 0,
    Forge = 1,
    Cauldron = 2,
    LiteLoader = 3,
    Fabric = 4,
    Quilt = 5,
    NeoForge = 6
}

#[derive(Deserialize)]
// https://docs.curseforge.com/#tocS_Mod
pub struct CurseForgeMod {
    #[serde(rename(deserialize = "id"))]
    pub mod_id: u32,
    pub slug: String,
    pub name: String,
    pub links: CurseForgeModLinks,
    #[serde(rename(deserialize = "classId"))]
    pub class_id: u32,
    #[serde(rename(deserialize = "mainFileId"))]
    pub main_file_id: u32,
    #[serde(rename(deserialize = "latestFiles"))]
    pub latest_files: u32,
    #[serde(rename(deserialize = "allowModDistribution"))]
    pub allow_mod_distribution: bool

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

#[derive(Deserialize)]
pub struct CurseForgeFingerprintMatches {
    #[serde(rename(deserialize = "isCacheBuilt"))]
    pub is_cache_built: bool,
    #[serde(rename(deserialize = "exactMatches"))]
    pub exact_matches: Vec<CurseForgeFingerprintMatch>,
    #[serde(rename(deserialize = "exactFingerprints"))]
    pub exact_fingerprints: Vec<u32>
}

#[derive(Deserialize)]
pub struct CurseForgeFingerprintMatch {
    #[serde(rename(deserialize = "id"))]
    pub match_id: u32,
    pub file: CurseForgeFile,
    #[serde(rename(deserialize = "latestFiles"))]
    pub latest_files: Vec<CurseForgeFile>
}
