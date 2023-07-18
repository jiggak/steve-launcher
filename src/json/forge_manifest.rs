use serde::Deserialize;

use super::{AssetDownload, ForgeVersionRequires};

#[derive(Deserialize)]
pub struct ForgeManifest {
    #[serde(rename(deserialize = "formatVersion"))]
    pub format_version: u8,
    pub libraries: Vec<ForgeLibrary>,
    #[serde(rename(deserialize = "mainClass"))]
    pub main_class: String,
    #[serde(rename(deserialize = "mavenFiles"))]
    pub maven_files: Vec<ForgeLibrary>,
    #[serde(rename(deserialize = "minecraftArguments"))]
    pub minecraft_arguments: String,
    pub name: String,
    pub order: u8,
    #[serde(rename(deserialize = "releaseTime"))]
    pub release_time: String,
    pub requires: Vec<ForgeVersionRequires>,
    pub uid: String,
    pub version: String
}

#[derive(Deserialize)]
pub struct ForgeLibrary {
    pub name: String,
    pub downloads: ForgeDownloads
}

#[derive(Deserialize)]
pub struct ForgeDownloads {
    pub artifact: ForgeArtifact
}

#[derive(Deserialize)]
pub struct ForgeArtifact {
    // ForgeWrapper-mmc2.jar and forge-1.19.4-45.1.6-installer.jar
    // don't have path properties, I have no idea why
    pub path: Option<String>,
    #[serde(flatten)]
    pub download: AssetDownload
}

impl ForgeArtifact {
    /// Returns `path` field or extracts path from `url` when `path` field is `None`
    pub fn asset_path(&self) -> String {
        let url = url::Url::parse(self.download.url.as_str()).unwrap();

        match &self.path {
            Some(path) => path.clone(),
            None => if url.path().starts_with("/maven/") {
                url.path().strip_prefix("/maven/").unwrap().to_string()
            } else {
                url.path().strip_prefix("/").unwrap().to_string()
            }
        }
    }
}
