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

use crate::Error;
use super::{AssetDownload, ForgeVersionRequires};

#[derive(Deserialize)]
pub struct ForgeManifest {
    #[serde(rename(deserialize = "+traits"))]
    pub traits: Option<Vec<String>>,
    #[serde(rename(deserialize = "+tweakers"))]
    pub tweakers: Option<Vec<String>>,
    #[serde(rename(deserialize = "formatVersion"))]
    pub format_version: u8,
    pub libraries: Vec<ForgeLibrary>,
    #[serde(rename(deserialize = "mainClass"))]
    pub main_class: String,
    #[serde(rename(deserialize = "mavenFiles"))]
    pub maven_files: Option<Vec<ForgeLibrary>>,
    #[serde(rename(deserialize = "minecraftArguments"))]
    pub minecraft_arguments: Option<String>,
    pub name: String,
    pub order: u8,
    #[serde(rename(deserialize = "releaseTime"))]
    pub release_time: String,
    pub requires: Vec<ForgeVersionRequires>,
    pub uid: String,
    pub version: String
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ForgeLibrary {
    Downloads {
        name: String,
        downloads: ForgeDownloads
    },
    Url {
        name: String,
        url: Option<String>
    }
}

impl ForgeLibrary {
    pub fn asset_path(&self) -> String {
        match self {
            ForgeLibrary::Downloads { downloads, .. } => downloads.artifact.asset_path(),
            ForgeLibrary::Url { name, .. } => name_to_path(name).unwrap()
        }
    }

    pub fn download_url(&self) -> String {
        match self {
            ForgeLibrary::Downloads { downloads, .. } => downloads.artifact.download.url.clone(),
            ForgeLibrary::Url { url, .. } => match url {
                Some(url) => format!("{url}/{path}", path = self.asset_path()),
                None => format!("https://libraries.minecraft.net/{path}", path = self.asset_path())
            }
        }
    }
}

/// Turns maven style name into library path
pub fn name_to_path(name: &str) -> Result<String, Error> {
    let mut parts = name.split(":");

    let err = format!("Unexpected library name '{}'", name);

    let (group_id, artifact_id, version, classifier) = (
        parts.next().ok_or(Error::new(err.as_str()))?,
        parts.next().ok_or(Error::new(err.as_str()))?,
        parts.next().ok_or(Error::new(err.as_str()))?,
        parts.next().map_or("".to_string(), |c| format!("-{c}"))
    );

    let file_name = format!("{artifact_id}-{version}{classifier}.jar");

    let mut path: Vec<_> = group_id.split(".").collect();
    path.extend([artifact_id, version, file_name.as_str()]);

    Ok(path.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_to_path_generic() {
        let result = name_to_path("org.ow2.asm:asm-tree:9.2").unwrap();
        assert_eq!(result, "org/ow2/asm/asm-tree/9.2/asm-tree-9.2.jar");
    }

    #[test]
    fn name_to_path_forge() {
        let result = name_to_path("net.minecraftforge:forge:1.19.4-45.1.0:universal").unwrap();
        assert_eq!(result, "net/minecraftforge/forge/1.19.4-45.1.0/forge-1.19.4-45.1.0-universal.jar");
    }
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
            None => {
                // strip "/maven/" from files.prismlauncher.org URL's
                if url.path().starts_with("/maven/") {
                    url.path().strip_prefix("/maven/").unwrap().to_string()
                // strip "/" from mavan.minecraftforge.net URL's
                } else {
                    url.path().strip_prefix("/").unwrap().to_string()
                }
            }
        }
    }
}
