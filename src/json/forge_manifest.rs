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

    #[serde(flatten)]
    pub dist: ForgeDistribution,

    pub name: String,
    #[serde(rename(deserialize = "releaseTime"))]
    pub release_time: String,
    pub requires: Vec<ForgeVersionRequires>,
    pub uid: String,
    pub version: String
}

impl ForgeManifest {
    pub fn get_minecraft_version(&self) -> Result<String, Error> {
        self.requires.iter()
            .find(|r| r.uid == "net.minecraft")
            .map(|r| r.equals.clone())
            .ok_or(Error::ForgeRequiresNotFound)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ForgeDistribution {
    Current {
        libraries: Vec<ForgeLibrary>,
        #[serde(rename(deserialize = "mainClass"))]
        main_class: String,
        #[serde(rename(deserialize = "mavenFiles"))]
        maven_files: Option<Vec<ForgeLibrary>>,
        #[serde(rename(deserialize = "minecraftArguments"))]
        minecraft_arguments: Option<String>
    },
    Legacy {
        #[serde(rename(deserialize = "jarMods"))]
        jar_mods: Vec<ForgeLibrary>,
        fml_libs: Option<Vec<ForgeLibrary>>
    }
}

impl ForgeDistribution {
    pub fn get_installer_lib(&self) -> Option<&ForgeLibrary> {
        match self {
            ForgeDistribution::Current { maven_files: Some(maven_files), .. } => {
                let installer_lib = maven_files.iter()
                    .find(|f| f.name().ends_with(":installer"));
                return installer_lib;
            },
            _ => None
        }
    }
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
    pub fn name(&self) -> &str {
        match self {
            ForgeLibrary::Downloads { name, .. } => &name,
            ForgeLibrary::Url { name, .. } => &name
        }
    }

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

    pub fn fml_libs_1_3() -> Vec<Self> {
        serde_json::from_str(include_str!("fml_libs_1.3.json")).unwrap()
    }

    pub fn fml_libs_1_4() -> Vec<Self> {
        serde_json::from_str(include_str!("fml_libs_1.4.json")).unwrap()
    }

    pub fn fml_libs_1_5(mc_version: &str) -> Vec<Self> {
        let mut libs: Vec<Self> = serde_json::from_str(include_str!("fml_libs_1.5.json")).unwrap();

        if mc_version == "1.5" {
            libs.push(serde_json::from_str(include_str!("fml_deobf_1.5.0.json")).unwrap());
        } else if mc_version == "1.5.1" {
            libs.push(serde_json::from_str(include_str!("fml_deobf_1.5.1.json")).unwrap());
        } else if mc_version == "1.5.2" {
            libs.push(serde_json::from_str(include_str!("fml_deobf_1.5.2.json")).unwrap());
        } else {
            panic!("Expected Minecraft version 1.5.[0-2], found {}", mc_version);
        }

        libs
    }
}

/// Turns maven style name into library path
pub fn name_to_path(name: &str) -> Result<String, Error> {
    let mut parts = name.split(':');

    let (group_id, artifact_id, version, classifier) = (
        parts.next().ok_or(Error::InvalidLibraryName(name.to_string()))?,
        parts.next().ok_or(Error::InvalidLibraryName(name.to_string()))?,
        parts.next().ok_or(Error::InvalidLibraryName(name.to_string()))?,
        parts.next().map_or("".to_string(), |c| format!("-{c}"))
    );

    let file_name = format!("{artifact_id}-{version}{classifier}.jar");

    let mut path: Vec<_> = group_id.split('.').collect();
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
                    url.path().strip_prefix('/').unwrap().to_string()
                }
            }
        }
    }
}
