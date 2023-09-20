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
use std::collections::HashMap;

use crate::{env, rules::RulesMatch};

#[derive(Deserialize)]
pub struct GameManifest {
    pub arguments: Option<GameArgsIndex>,
    #[serde(rename(deserialize = "assetIndex"))]
    pub asset_index: GameAssetIndex,
    pub assets: String,
    #[serde(rename(deserialize = "complianceLevel"))]
    pub compliance_level: Option<u8>,
    pub downloads: HashMap<String, AssetDownload>,
    pub id: String,
    #[serde(rename(deserialize = "javaVersion"))]
    pub java_version: Option<GameJavaVersion>,
    pub libraries: Vec<GameLibrary>,
    pub logging: Option<GameLogging>,
    #[serde(rename(deserialize = "mainClass"))]
    pub main_class: String,
    #[serde(rename(deserialize = "minecraftArguments"))]
    pub minecraft_arguments: Option<String>,
    #[serde(rename(deserialize = "minimumLauncherVersion"))]
    pub minimum_launcher_version: u8,
    #[serde(rename(deserialize = "releaseTime"))]
    pub release_time: String,
    pub time: String,
    #[serde(rename(deserialize = "type"))]
    pub release_type: String
}

#[derive(Deserialize)]
pub struct GameArgsIndex {
    pub game: GameArgs,
    pub jvm: GameArgs
}

#[derive(Deserialize)]
#[serde(from = "GameArgsRaw")]
pub struct GameArgs(pub Vec<GameArg>);

impl GameArgs {
    pub fn matched_args(&self) -> impl Iterator<Item = String> + '_ {
        self.0.iter()
            .filter(|arg| arg.rules.matches())
            .flat_map(|arg| {
                match &arg.value {
                    GameArgValue::Single(v) => vec![v.clone()],
                    GameArgValue::Many(v) => v.to_vec()
                }
            })
    }
}

#[derive(Deserialize)]
struct GameArgsRaw(Vec<GameArgTypes>);

#[derive(Deserialize)]
#[serde(untagged)]
enum GameArgTypes {
    String(String),
    GameArg(GameArg)
}

#[derive(Deserialize, Clone)]
pub struct GameArg {
    pub value: GameArgValue,
    pub rules: Vec<GameArgRule>
}

impl GameArg {
    fn new<S: Into<String>>(v: S) -> Self {
        Self {
            value: GameArgValue::Single(v.into()),
            rules: Vec::new()
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum GameArgValue {
    Single(String),
    Many(Vec<String>)
}

#[derive(Deserialize, Clone)]
pub struct GameArgRule {
    pub action: String,
    pub features: Option<HashMap<String, bool>>,
    pub os: Option<OsProperties>
}

#[derive(Deserialize, Clone)]
pub struct OsProperties {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>
}

impl From<GameArgsRaw> for GameArgs {
    fn from(args: GameArgsRaw) -> Self {
        GameArgs(args.0.iter().map(|elem| {
            match elem {
                GameArgTypes::String(v) => GameArg::new(v),
                GameArgTypes::GameArg(v) => v.clone()
            }
        }).collect())
    }
}

#[derive(Deserialize)]
pub struct GameAssetIndex {
    pub id: String,
    #[serde(flatten)]
    pub download: AssetDownload,
    #[serde(rename(deserialize = "totalSize"))]
    pub total_size: u64
}

#[derive(Deserialize)]
pub struct AssetDownload {
    pub sha1: String,
    pub size: u32,
    pub url: String
}

#[derive(Deserialize)]
pub struct GameJavaVersion {
    pub component: String,
    #[serde(rename(deserialize = "majorVersion"))]
    pub major_version: u8
}

#[derive(Deserialize)]
pub struct GameLibrary {
    pub downloads: GameLibraryDownloads,
    pub extract: Option<GameLibraryExtract>,
    pub name: String,
    pub natives: Option<HashMap<String, String>>,
    pub rules: Option<Vec<GameLibraryRule>>,
}

impl GameLibrary {
    pub fn has_rules_match(&self) -> bool {
        match &self.rules {
            Some(rules) => rules.matches(),

            // lib matches if rules don't exist
            None => true
        }
    }

    pub fn natives_artifact(&self) -> Result<Option<&GameLibraryArtifact>, GameLibError> {
        Ok(match &self.natives {
            Some(natives) => {
                let host_os = env::get_host_os();

                let natives_key = natives.get(host_os)
                    .ok_or(GameLibError::OsNotFound {
                        lib_name: self.name.to_string(),
                        os_name: host_os.to_string()
                    })?;

                let classifiers = self.downloads.classifiers.as_ref()
                    .ok_or(GameLibError::ClassifiersNotFound(self.name.to_string()))?;

                let artifact = classifiers.get(natives_key)
                    .ok_or(GameLibError::ClassifierNativeKeyNotFound {
                        lib_name: self.name.to_string(),
                        natives_key: natives_key.to_string()
                    })?;

                Some(artifact)
            }

            None => None
        })
    }

    pub fn artifacts_for_download(&self) -> Result<Vec<&GameLibraryArtifact>, GameLibError> {
        let artifacts = [
            self.downloads.artifact.as_ref(),
            self.natives_artifact()?
        ];

        let artifacts: Vec<&GameLibraryArtifact> = artifacts.iter()
            .filter_map(|a| *a)
            .collect();

        if artifacts.is_empty() {
            Err(GameLibError::UnhandledDownload(self.name.to_string()))
        } else {
            Ok(artifacts)
        }
    }

    pub fn log4j_api_2_17_1() -> Self {
        serde_json::from_str(r#"
        {
            "downloads": {
               "artifact": {
                  "path": "org/apache/logging/log4j/log4j-api/2.17.1/log4j-api-2.17.1.jar",
                  "sha1": "d771af8e336e372fb5399c99edabe0919aeaf5b2",
                  "size": 301872,
                  "url": "https://repo1.maven.org/maven2/org/apache/logging/log4j/log4j-api/2.17.1/log4j-api-2.17.1.jar"
               }
            },
            "name": "org.apache.logging.log4j:log4j-api:2.17.1"
        }"#).unwrap()
    }

    pub fn log4j_core_2_17_1() -> Self {
        serde_json::from_str(r#"
        {
            "downloads": {
               "artifact": {
                  "path": "org/apache/logging/log4j/log4j-core/2.17.1/log4j-core-2.17.1.jar",
                  "sha1": "779f60f3844dadc3ef597976fcb1e5127b1f343d",
                  "size": 1790452,
                  "url": "https://repo1.maven.org/maven2/org/apache/logging/log4j/log4j-core/2.17.1/log4j-core-2.17.1.jar"
               }
            },
            "name": "org.apache.logging.log4j:log4j-core:2.17.1"
        }"#).unwrap()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GameLibError {
    #[error("OS name '{os_name}' not found in lib {lib_name} natives")]
    OsNotFound { lib_name: String, os_name: String },
    #[error("Lib {0} missing 'classifiers' object")]
    ClassifiersNotFound(String),
    #[error("Expected key '{natives_key}' in lib {lib_name} classifiers")]
    ClassifierNativeKeyNotFound { lib_name: String, natives_key: String },
    #[error("Hnhandled download for {0}")]
    UnhandledDownload(String)
}

#[derive(Deserialize)]
pub struct GameLibraryDownloads {
    pub artifact: Option<GameLibraryArtifact>,
    pub classifiers: Option<HashMap<String, GameLibraryArtifact>>
}

#[derive(Deserialize)]
pub struct GameLibraryArtifact {
    pub path: String,
    #[serde(flatten)]
    pub download: AssetDownload
}

#[derive(Deserialize)]
pub struct GameLibraryExtract {
    pub exclude: Vec<String>
}

#[derive(Deserialize)]
pub struct GameLibraryRule {
    pub action: String,
    pub os: Option<OsProperties>
}

#[derive(Deserialize)]
pub struct GameLogging {
    pub client: GameLoggingClient
}

#[derive(Deserialize)]
pub struct GameLoggingClient {
    pub argument: String,
    pub file: GameLoggingArtifact,
    #[serde(rename(deserialize = "type"))]
    pub file_type: String
}

#[derive(Deserialize)]
pub struct GameLoggingArtifact {
    pub id: String,
    #[serde(flatten)]
    pub download: AssetDownload
}
