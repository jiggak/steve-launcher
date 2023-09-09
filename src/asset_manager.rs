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

use semver::Version;
use std::{collections::HashMap, fs, path::Path, path::PathBuf};
use std::error::Error as StdError;

use crate::{asset_client::AssetClient, env, Error, Progress};
use crate::json::{
    AssetDownload, AssetManifest, ForgeLibrary, ForgeManifest, ForgeManifestVariant,
    GameLibrary, GameLibraryArtifact, GameManifest
};

pub struct AssetManager {
    client: AssetClient,
    assets_dir: PathBuf,
    cache_dir: PathBuf,
    libs_dir: PathBuf
}

impl AssetManager {
    pub fn new() -> Result<Self, Box<dyn StdError>> {
        let manager = AssetManager {
            client: AssetClient::new(),
            assets_dir: env::get_assets_dir(),
            cache_dir: env::get_cache_dir(),
            libs_dir: env::get_libs_dir()
        };

        fs::create_dir_all(manager.objects_dir())?;
        fs::create_dir_all(manager.indexes_dir())?;
        fs::create_dir_all(manager.versions_dir())?;

        Ok(manager)
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.assets_dir.join("objects")
    }

    pub fn indexes_dir(&self) -> PathBuf {
        self.assets_dir.join("indexes")
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.cache_dir.join("versions")
    }

    pub fn virtual_assets_dir(&self, asset_index_id: &str) -> PathBuf {
        self.assets_dir.join("virtual").join(asset_index_id)
    }

    pub async fn get_game_manifest(&self, mc_version: &str) -> Result<GameManifest, Box<dyn StdError>> {
        let version_file_path = self.versions_dir()
            .join(format!("{mc_version}.json"));

        if !version_file_path.exists() {
            let game_manifest_json = self.client.get_game_manifest_json(mc_version).await?;

            fs::write(&version_file_path, game_manifest_json)?;
        }

        let version_file = fs::File::open(version_file_path)?;
        let mut game_manifest: GameManifest = serde_json::from_reader(version_file)?;

        apply_lib_overrides(&mut game_manifest)?;

        Ok(game_manifest)
    }

    pub async fn get_forge_manifest(&self, forge_version: &str) -> Result<ForgeManifest, Box<dyn StdError>> {
        let version_file_path = self.versions_dir()
            .join(format!("forge_{forge_version}.json"));

        if !version_file_path.exists() {
            let json = self.client.get_forge_manifest_json(forge_version).await?;

            fs::write(&version_file_path, json)?;
        }

        let version_file = fs::File::open(version_file_path)?;
        let forge_manifest: ForgeManifest = serde_json::from_reader(version_file)?;

        Ok(forge_manifest)
    }

    pub async fn get_asset_manfiest(&self, game_manifest: &GameManifest) -> Result<AssetManifest, Box<dyn StdError>> {
        let index_file_path = self.indexes_dir()
            .join(format!("{ver}.json", ver = game_manifest.asset_index.id));

        if index_file_path.exists() {
            let index_file = fs::File::open(&index_file_path)?;
            return Ok(serde_json::from_reader(index_file)?);
        }

        let asset_index_url = game_manifest.asset_index.download.url.as_str();
        let asset_manifest = self.client.get_asset_manfiest(asset_index_url).await?;

        let index_file = fs::File::create(index_file_path)?;
        serde_json::to_writer(index_file, &asset_manifest)?;

        Ok(asset_manifest)
    }

    pub async fn download_assets(&self,
        asset_manifest: &AssetManifest,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        progress.begin("Downloading assets", asset_manifest.objects.len());

        for (i, (_, obj)) in asset_manifest.objects.iter().enumerate() {
            progress.advance(i + 1);
            self.download_asset(&obj.hash).await?;
        }

        progress.end();

        Ok(())
    }

    async fn download_asset(&self, hash: &str) -> Result<(), Box<dyn StdError>> {
        // first 2 chars of hash is used for directory of objects
        let hash_prefix = &hash[0..2];

        let object_file = self.objects_dir()
            .join(hash_prefix)
            .join(hash);

        // skip download if object file already exists
        if object_file.exists() {
            return Ok(());
        }

        let url = format!("https://resources.download.minecraft.net/{hash_prefix}/{hash}");

        self.client.download_file(&url, &object_file).await
    }

    pub async fn download_libraries(&self,
        game_manifest: &GameManifest,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        let client = game_manifest.downloads.get("client")
            .ok_or(Error::new("Missing 'client' key in downloads object"))?;

        let client_path = get_client_jar_path(&game_manifest.id);
        let mut lib_downloads: Vec<(&str, &String)> = vec![
            (client_path.as_str(), &client.url)
        ];

        lib_downloads.extend(
            game_manifest.libraries.iter()
                .filter(|lib| lib.has_rules_match())
                .flat_map(|lib| {
                    // FIXME I think this can be done in a more Rust'y way
                    let mut result = vec![];

                    if let Some(artifact) = &lib.downloads.artifact {
                        result.push(artifact);
                    }

                    if let Some(natives) = lib.natives_artifact() {
                        result.push(natives);
                    }

                    if result.is_empty() {
                        panic!("unhandled download for {}", lib.name);
                    }

                    return result;
                })
                .map(|a| (a.path.as_str(), &a.download.url))
        );

        progress.begin("Downloading libraries", lib_downloads.len());

        for (i, (path, url)) in lib_downloads.iter().enumerate() {
            progress.advance(i + 1);
            self.download_library(path, url).await?;
        }

        progress.end();

        Ok(())
    }

    pub async fn download_forge_libraries(&self,
        forge_manifest: &ForgeManifest,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        // let srcs: dyn Iterator<Item = ForgeLibrary> = match forge_manifest.maven_files {
        //     Some(maven_files) => forge_manifest.libraries.iter().chain(maven_files.iter()),
        //     None => forge_manifest.libraries.iter()
        // };

        let mut downloads: Vec<&ForgeLibrary> = vec![];

        match &forge_manifest.variant {
            ForgeManifestVariant::JarMod { jar_mods } => {
                downloads.extend(jar_mods.iter());
            },
            ForgeManifestVariant::NonJarMod { libraries, maven_files, .. } => {
                downloads.extend(libraries.iter());

                if let Some(maven_files) = maven_files {
                    downloads.extend(maven_files.iter());
                }
            }
        }

        progress.begin("Downloading forge libraries", downloads.len());

        for (i, (path, url)) in downloads.iter().map(|lib| (lib.asset_path(), lib.download_url())).enumerate() {
            progress.advance(i + 1);
            self.download_library(&path, &url).await?;
        }

        progress.end();

        Ok(())
    }

    async fn download_library(&self, path: &str, url: &str) -> Result<(), Box<dyn StdError>> {
        let lib_file = self.libs_dir.join(path);

        // skip download if lib file already exists
        if lib_file.exists() {
            return Ok(());
        }

        self.client.download_file(url, &lib_file).await
    }

    pub fn copy_resources(&self,
        asset_manifest: &AssetManifest,
        target_dir: &Path,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        progress.begin("Copy resources", asset_manifest.objects.len());

        for (i, (path, obj)) in asset_manifest.objects.iter().enumerate() {
            let object_path = self.objects_dir()
                .join(&obj.hash[0..2])
                .join(&obj.hash);

            let resource_path = target_dir.join(path);

            if !resource_path.exists() {
                fs::create_dir_all(resource_path.parent().unwrap())?;
                fs::copy(object_path, resource_path)?;
            }

            progress.advance(i + 1);
        }

        progress.end();

        Ok(())
    }

    pub fn extract_natives(self,
        game_manifest: &GameManifest,
        target_dir: &Path,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        let native_libs: Vec<_> = game_manifest.libraries.iter()
            .filter(|lib| lib.has_rules_match())
            .filter_map(|lib| lib.natives_artifact())
            .collect();

        progress.begin("Extracting native jars", native_libs.len());

        for (i, lib) in native_libs.iter().enumerate() {
            let lib_file = self.libs_dir.join(&lib.path);
            zip_extract::extract(fs::File::open(lib_file)?, target_dir, false)?;
            progress.advance(i + 1);
        }

        progress.end();

        Ok(())
    }
}

pub fn get_client_jar_path(mc_version: &str) -> String {
    format!("com/mojang/minecraft/{mc_version}/minecraft-{mc_version}-client.jar")
}

pub fn dedup_libs(libs: &Vec<String>) -> Result<Vec<&String>, Box<dyn StdError>> {
    let mut lib_map = HashMap::new();

    // native jars have the same artifact path and version as their
    // companion jar and will get incorrectly removed in the dedup process
    // this naive approach splits natives jars, assuming these jars will always
    // have the substring "natives" in the path, and includes them after the
    // dedup process is complete
    let (natives, non_natives): (Vec<_>, Vec<_>) = libs.iter()
        .partition(|l| l.contains("natives"));

    for path in non_natives {
        let mut parts = path.rsplitn(3, "/");

        let err = format!("Unexpected library path '{}'", path);

        let (_, sversion, artifact_id) = (
            parts.next().ok_or(Error::new(err.as_str()))?,
            parts.next().ok_or(Error::new(err.as_str()))?,
            parts.next().ok_or(Error::new(err.as_str()))?
        );

        // some paths don't have a valid version
        // e.g. "mmc2" -> io/github/zekerzhayard/ForgeWrapper/mmc2/ForgeWrapper-mmc2.jar
        // for these, lets invent some meaningless version instead of crashing
        // fingers crossed these types of libs will never have duplicates
        let version = lenient_semver::parse(sversion);
        let version = if version.is_err() {
            Version::new(9, 9, 9)
        } else {
            version.unwrap()
        };

        if let Some((existing_version, _)) = lib_map.get(artifact_id) {
            if *existing_version < version {
                lib_map.insert(artifact_id, (version, path));
            }
        } else {
            lib_map.insert(artifact_id, (version, path));
        }
    }

    Ok(lib_map.values()
        .map(|(_, path)| *path)
        .chain(natives)
        .collect())
}

// This logic is taken from PrismLauncher meta data generator
// https://github.com/PrismLauncher/meta/blob/44d7582f91ae87fdf9d99ef8715e6a5562b5a715/generateMojang.py
// I understand this is in response to the nasty log4j vulnerability.
// What I don't understand is why the PrismLauncher Forge meta data generator
// doesn't include all of Forge's dependancies.
// e.g. https://meta.prismlauncher.org/v1/net.minecraftforge/36.2.39.json
// This doesn't include log4j at all, but forge 36.2.39 installer does include
// log4j 2.15 in its libraries list.
fn apply_lib_overrides(game_manifest: &mut GameManifest) -> Result<(), Error> {
    let ver_2_17_1 = Version::parse("2.17.1").unwrap();
    let ver_2_0 = Version::parse("2.0.0").unwrap();

    for l in &mut game_manifest.libraries {
        let lib_name = l.name.clone();
        let mut parts = lib_name.split(":");

        let err = format!("Unexpected library name '{}'", l.name);

        let (group_id, name, sversion) = (
            parts.next().ok_or(Error::new(err.as_str()))?,
            parts.next().ok_or(Error::new(err.as_str()))?,
            parts.next().ok_or(Error::new(err.as_str()))?
        );

        if group_id != "org.apache.logging.log4j" {
            continue;
        }

        let version = lenient_semver::parse(sversion)
            .map_err(|_| Error::new(format!("Unable to parse log4j SemVer '{sversion}'")))?;

        if name == "log4j-api" && version > ver_2_0 && version < ver_2_17_1 {
            *l =  GameLibrary::new_artifact_download(
                "org.apache.logging.log4j:log4j-api:2.17.1",
                GameLibraryArtifact {
                    path: "org/apache/logging/log4j/log4j-api/2.17.1/log4j-api-2.17.1.jar".into(),
                    download: AssetDownload {
                        sha1: "d771af8e336e372fb5399c99edabe0919aeaf5b2".into(),
                        size: 301872,
                        url: "https://repo1.maven.org/maven2/org/apache/logging/log4j/log4j-api/2.17.1/log4j-api-2.17.1.jar".into()
                    }
                }
            );
        }

        if name == "log4j-core" && version > ver_2_0 && version < ver_2_17_1 {
            *l = GameLibrary::new_artifact_download(
                "org.apache.logging.log4j:log4j-core:2.17.1",
                GameLibraryArtifact {
                    path: "org/apache/logging/log4j/log4j-core/2.17.1/log4j-core-2.17.1.jar".into(),
                    download: AssetDownload {
                        sha1: "779f60f3844dadc3ef597976fcb1e5127b1f343d".into(),
                        size: 1790452,
                        url: "https://repo1.maven.org/maven2/org/apache/logging/log4j/log4j-core/2.17.1/log4j-core-2.17.1.jar".into()
                    }
                }
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_libs_simple() {
        let input = vec!["a/b/1.2.3/b-1.2.3.jar".to_string(), "a/b/1.2.4/b-1.2.4.jar".to_string()];
        let result: Vec<_> = dedup_libs(&input).unwrap();
        assert_eq!(result, vec!["a/b/1.2.4/b-1.2.4.jar"]);
    }

    #[test]
    fn dedup_libs_semver_order() {
        let input = vec!["a/b/45.1.2/b-45.1.2.jar".to_string(), "a/b/45.1.16/b-45.1.16.jar".to_string()];
        let result: Vec<_> = dedup_libs(&input).unwrap();
        assert_eq!(result, vec!["a/b/45.1.16/b-45.1.16.jar"]);
    }

    #[test]
    fn dedup_libs_wacky_ver() {
        let input = vec![
            "net/minecraftforge/forge/1.7.10-10.13.4.1566-1.7.10/forge-1.7.10-10.13.4.1566-1.7.10-universal.jar".to_string(),
            "net/minecraftforge/forge/1.7.10-10.13.4.1614-1.7.10/forge-1.7.10-10.13.4.1614-1.7.10-universal.jar".to_string()
        ];
        let result: Vec<_> = dedup_libs(&input).unwrap();
        assert_eq!(result, vec!["net/minecraftforge/forge/1.7.10-10.13.4.1614-1.7.10/forge-1.7.10-10.13.4.1614-1.7.10-universal.jar"]);
    }

    #[test]
    fn dedup_keep_natives() {
        let input = vec![
            "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar".to_string(),
            "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-linux.jar".to_string()
        ];
        let result: Vec<_> = dedup_libs(&input).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn dedup_invalid_version() {
        let input = vec![
            "io/github/zekerzhayard/ForgeWrapper/mmc2/ForgeWrapper-mmc2.jar".to_string(),
            "org/ow2/asm/asm/9.5/asm-9.5.jar".to_string()
        ];
        let result: Vec<_> = dedup_libs(&input).unwrap();
        assert_eq!(result.len(), 2);
    }
}
