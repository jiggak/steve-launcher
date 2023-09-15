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

    pub fn fml_libs_1_3() -> Vec<Self> {
        serde_json::from_str(r#"
        [
            {
                "name": "fmllibs:argo:2.25",
                "downloads": {
                    "artifact": {
                        "path": "argo-2.25.jar",
                        "sha1": "bb672829fde76cb163004752b86b0484bd0a7f4b",
                        "size": 123642,
                        "url": "https://files.prismlauncher.org/fmllibs/argo-2.25.jar"
                    }
                }
            },
            {
                "name": "fmllibs:guava:12.0.1",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/guava-12.0.1.jar",
                        "sha1": "b8e78b9af7bf45900e14c6f958486b6ca682195f",
                        "size": 1795932,
                        "url": "https://files.prismlauncher.org/fmllibs/guava-12.0.1.jar"
                    }
                }
            },
            {
                "name": "fmllibs:asm-all:4.0",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/asm-all-4.0.jar",
                        "sha1": "98308890597acb64047f7e896638e0d98753ae82",
                        "size": 212767,
                        "url": "https://files.prismlauncher.org/fmllibs/asm-all-4.0.jar"
                    }
                }
            }
        ]"#).unwrap()
    }

    pub fn fml_libs_1_4() -> Vec<Self> {
        serde_json::from_str(r#"
        [
            {
                "name": "fmllibs:argo:2.25",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/argo-2.25.jar",
                        "sha1": "bb672829fde76cb163004752b86b0484bd0a7f4b",
                        "size": 123642,
                        "url": "https://files.prismlauncher.org/fmllibs/argo-2.25.jar"
                    }
                }
            },
            {
                "name": "fmllibs:guava:12.0.1",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/guava-12.0.1.jar",
                        "sha1": "b8e78b9af7bf45900e14c6f958486b6ca682195f",
                        "size": 1795932,
                        "url": "https://files.prismlauncher.org/fmllibs/guava-12.0.1.jar"
                    }
                }
            },
            {
                "name": "fmllibs:asm-all:4.0",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/asm-all-4.0.jar",
                        "sha1": "98308890597acb64047f7e896638e0d98753ae82",
                        "size": 212767,
                        "url": "https://files.prismlauncher.org/fmllibs/asm-all-4.0.jar"
                    }
                }
            },
            {
                "name": "fmllibs:bcprov-jdk15on:147",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/bcprov-jdk15on-147.jar",
                        "sha1": "b6f5d9926b0afbde9f4dbe3db88c5247be7794bb",
                        "size": 1997327,
                        "url": "https://files.prismlauncher.org/fmllibs/bcprov-jdk15on-147.jar"
                    }
                }
            }
        ]"#).unwrap()
    }

    pub fn fml_libs_1_5(mc_version: &str) -> Vec<Self> {
        let mut libs: Vec<Self> = serde_json::from_str(r#"
        [
            {
                "name": "fmllibs:argo-small:3.2",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/argo-small-3.2.jar",
                        "sha1": "58912ea2858d168c50781f956fa5b59f0f7c6b51",
                        "size": 91333,
                        "url": "https://files.prismlauncher.org/fmllibs/argo-small-3.2.jar"
                    }
                }
            },
            {
                "name": "fmllibs:guava:14.0:rc3",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/guava-14.0-rc3.jar",
                        "sha1": "931ae21fa8014c3ce686aaa621eae565fefb1a6a",
                        "size": 2189140,
                        "url": "https://files.prismlauncher.org/fmllibs/guava-14.0-rc3.jar"
                    }
                }
            },
            {
                "name": "fmllibs:asm-all:4.1",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/asm-all-4.1.jar",
                        "sha1": "054986e962b88d8660ae4566475658469595ef58",
                        "size": 214592,
                        "url": "https://files.prismlauncher.org/fmllibs/asm-all-4.1.jar"
                    }
                }
            },
            {
                "name": "fmllibs:bcprov-jdk15on:148",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/bcprov-jdk15on-148.jar",
                        "sha1": "960dea7c9181ba0b17e8bab0c06a43f0a5f04e65",
                        "size": 2318161,
                        "url": "https://files.prismlauncher.org/fmllibs/bcprov-jdk15on-148.jar"
                    }
                }
            },
            {
                "name": "fmllibs:scala-library",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/scala-library.jar",
                        "sha1": "458d046151ad179c85429ed7420ffb1eaf6ddf85",
                        "size": 7114640,
                        "url": "https://files.prismlauncher.org/fmllibs/scala-library.jar"
                    }
                }
            }
        ]"#).unwrap();

        if mc_version == "1.5" {
            libs.push(serde_json::from_str(r#"
            {
                "name": "fmllibs:deobfuscation_data:1.5",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/deobfuscation_data_1.5.zip",
                        "sha1": "5f7c142d53776f16304c0bbe10542014abad6af8",
                        "size": 200547,
                        "url": "https://files.prismlauncher.org/fmllibs/deobfuscation_data_1.5.zip"
                    }
                }
            }
            "#).unwrap());
        } else if mc_version == "1.5.1" {
            libs.push(serde_json::from_str(r#"
            {
                "name": "fmllibs:deobfuscation_data:1.5.1",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/deobfuscation_data_1.5.1.zip",
                        "sha1": "22e221a0d89516c1f721d6cab056a7e37471d0a6",
                        "size": 200886,
                        "url": "https://files.prismlauncher.org/fmllibs/deobfuscation_data_1.5.1.zip"
                    }
                }
            }
            "#).unwrap());
        } else if mc_version == "1.5.2" {
            libs.push(serde_json::from_str(r#"
            {
                "name": "fmllibs:deobfuscation_data:1.5.2",
                "downloads": {
                    "artifact": {
                        "path": "fmllibs/deobfuscation_data_1.5.2.zip",
                        "sha1": "446e55cd986582c70fcf12cb27bc00114c5adfd9",
                        "size": 201404,
                        "url": "https://files.prismlauncher.org/fmllibs/deobfuscation_data_1.5.2.zip"
                    }
                }
            }
            "#).unwrap());
        } else {
            panic!("Expected Minecraft version 1.5.x, found {}", mc_version);
        }

        libs
    }
}

/// Turns maven style name into library path
pub fn name_to_path(name: &str) -> Result<String, Error> {
    let mut parts = name.split(':');

    let err = format!("Unexpected library name '{}'", name);

    let (group_id, artifact_id, version, classifier) = (
        parts.next().ok_or(Error::new(err.as_str()))?,
        parts.next().ok_or(Error::new(err.as_str()))?,
        parts.next().ok_or(Error::new(err.as_str()))?,
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
