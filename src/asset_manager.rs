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

use anyhow::{Context, Result};
use semver::{Version, VersionReq};
use std::{collections::HashMap, fs, path::Path, path::PathBuf};

use crate::{asset_client::AssetClient, env, Error, Progress, zip};
use crate::json::{
    AssetManifest, ForgeDistribution, ForgeLibrary, ForgeManifest, GameLibrary,
    GameManifest, ModLoader
};

pub struct AssetManager {
    client: AssetClient,
    assets_dir: PathBuf,
    cache_dir: PathBuf,
    libs_dir: PathBuf
}

impl AssetManager {
    pub fn new() -> Result<Self> {
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

    pub async fn get_game_manifest(&self, mc_version: &str) -> Result<GameManifest> {
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

    pub async fn get_loader_manifest(&self, mod_loader: &ModLoader) -> Result<ForgeManifest> {
        let file_name = format!("{name}_{ver}.json",
            name = mod_loader.name.to_string(),
            ver = mod_loader.version
        );

        let version_file_path = self.versions_dir()
            .join(file_name);

        if !version_file_path.exists() {
            let json = self.client.get_loader_manifest_json(mod_loader).await?;

            fs::write(&version_file_path, json)?;
        }

        let version_file = fs::File::open(version_file_path)?;
        let mut forge_manifest: ForgeManifest = serde_json::from_reader(version_file)?;

        populate_fml_libs(&mut forge_manifest)?;

        Ok(forge_manifest)
    }

    pub async fn get_asset_manfiest(&self, game_manifest: &GameManifest) -> Result<AssetManifest> {
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
    ) -> Result<()> {
        progress.begin("Downloading assets", asset_manifest.objects.len());

        for (i, (_, obj)) in asset_manifest.objects.iter().enumerate() {
            progress.advance(i + 1);
            self.download_asset(&obj.hash).await?;
        }

        progress.end();

        Ok(())
    }

    async fn download_asset(&self, hash: &str) -> Result<()> {
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
    ) -> Result<()> {
        let client_path = get_client_jar_path(&game_manifest.id);
        let mut lib_downloads: Vec<(&str, &String)> = vec![
            (client_path.as_str(), &game_manifest.downloads.client.url)
        ];

        lib_downloads.extend(
            game_manifest.libraries.iter()
                .filter(|lib| lib.has_rules_match())
                // FIXME how to let this result error propagate?
                .flat_map(|lib| lib.artifacts_for_download().unwrap())
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

    pub async fn download_loader_libraries(&self,
        forge_manifest: &ForgeManifest,
        progress: &mut dyn Progress
    ) -> Result<()> {
        let mut downloads: Vec<&ForgeLibrary> = vec![];

        match &forge_manifest.dist {
            ForgeDistribution::Legacy { jar_mods, fml_libs } => {
                downloads.extend(jar_mods.iter());
                if let Some(fml_libs) = fml_libs {
                    downloads.extend(fml_libs.iter());
                }
            },
            ForgeDistribution::Current { libraries, maven_files, .. } => {
                downloads.extend(libraries.iter());

                if let Some(maven_files) = maven_files {
                    downloads.extend(maven_files.iter());
                }
            }
        }

        progress.begin("Downloading mod loader libraries", downloads.len());

        for (i, (path, url)) in downloads.iter().map(|lib| (lib.asset_path(), lib.download_url())).enumerate() {
            progress.advance(i + 1);
            self.download_library(&path, &url).await?;
        }

        progress.end();

        Ok(())
    }

    async fn download_library(&self, path: &str, url: &str) -> Result<()> {
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
    ) -> Result<()> {
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
    ) -> Result<()> {
        let native_libs: Vec<_> = game_manifest.libraries.iter()
            .filter(|lib| lib.has_rules_match())
            // FIXME how to let this result error propagate?
            .filter_map(|lib| lib.natives_artifact().unwrap())
            .collect();

        progress.begin("Extracting native jars", native_libs.len());

        for (i, lib) in native_libs.iter().enumerate() {
            let lib_file = self.libs_dir.join(&lib.path);
            zip::extract_zip(fs::File::open(lib_file)?, target_dir)?;
            progress.advance(i + 1);
        }

        progress.end();

        Ok(())
    }
}

/// Get path of minecraft client jar relative to shared libs directory
pub fn get_client_jar_path(mc_version: &str) -> String {
    format!("com/mojang/minecraft/{mc_version}/minecraft-{mc_version}-client.jar")
}

/// Make modded minecraft jar with forge, if it doesn't already exist, and
/// return the path of the modded jar
pub fn make_forge_modded_jar(
    mc_jar_path: &String, forge_version: &String, jar_mods: &Vec<ForgeLibrary>
) -> Result<PathBuf> {
    let modded_jar = format!("minecraft+forge-{}.jar", forge_version);
    let modded_jar_path = env::get_cache_dir().join(&modded_jar);
    if !modded_jar_path.exists() {
        // path to vanilla `minecraft.jar`
        let mc_jar_path = env::get_libs_dir().join(&mc_jar_path);

        // map forge jar_mods asset library paths
        let jar_mods: Vec<_> = jar_mods.iter()
            .map(|jar| env::get_libs_dir().join(jar.asset_path()))
            .collect();

        // create the modified `minecraft.jar`
        zip::make_modded_jar(
            &modded_jar_path,
            &mc_jar_path,
            jar_mods.iter().map(|p| p.as_path())
        )?;
    }

    Ok(modded_jar_path)
}

pub fn dedup_libs(libs: &[String]) -> Result<Vec<&String>> {
    let mut lib_map = HashMap::new();

    // native jars have the same artifact path and version as their
    // companion jar and will get incorrectly removed in the dedup process
    // this naive approach splits natives jars, assuming these jars will always
    // have the substring "natives" in the path, and includes them after the
    // dedup process is complete
    let (natives, non_natives): (Vec<_>, Vec<_>) = libs.iter()
        .partition(|l| l.contains("natives"));

    for path in non_natives {
        let mut parts = path.rsplitn(3, '/');

        let (_, sversion, artifact_id) = (
            parts.next().ok_or(Error::InvalidLibraryPath(path.clone()))?,
            parts.next().ok_or(Error::InvalidLibraryPath(path.clone()))?,
            parts.next().ok_or(Error::InvalidLibraryPath(path.clone()))?
        );

        // some paths don't have a valid version
        // e.g. "mmc2" -> io/github/zekerzhayard/ForgeWrapper/mmc2/ForgeWrapper-mmc2.jar
        // for these, lets invent some meaningless version instead of crashing
        // fingers crossed these types of libs will never have duplicates
        let version = lenient_semver::parse(sversion)
            .unwrap_or(Version::new(9, 9, 9));

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
fn apply_lib_overrides(game_manifest: &mut GameManifest) -> Result<()> {
    let range = VersionReq::parse(">2.0.0, <2.17.1").unwrap();

    for l in &mut game_manifest.libraries {
        let lib_name = l.name.clone();
        let mut parts = lib_name.split(':');

        let (group_id, name, sversion) = (
            parts.next().ok_or(Error::InvalidLibraryName(lib_name.clone()))?,
            parts.next().ok_or(Error::InvalidLibraryName(lib_name.clone()))?,
            parts.next().ok_or(Error::InvalidLibraryName(lib_name.clone()))?
        );

        if group_id != "org.apache.logging.log4j" {
            continue;
        }

        let version = lenient_semver::parse(sversion)
            .map_err(|_| Error::VersionParse { version: sversion.to_string() })
            .with_context(|| format!("Unable to parse log4j SemVer '{sversion}'"))?;

        if name == "log4j-api" && range.matches(&version) {
            *l =  GameLibrary::log4j_api_2_17_1();
        }

        if name == "log4j-core" && range.matches(&version) {
            *l = GameLibrary::log4j_core_2_17_1();
        }
    }

    Ok(())
}

fn populate_fml_libs(forge_manifest: &mut ForgeManifest) -> Result<()> {
    let mc_version = forge_manifest.get_minecraft_version()?;
    let mc_version_semver = lenient_semver::parse(&mc_version)
        .map_err(|_| Error::VersionParse { version: mc_version.clone() })
        .with_context(|| format!("Unable to parse forge SemVer '{mc_version}'"))?;

    let v1_4_range = VersionReq::parse(">=1.4.0, <1.5.0").unwrap();
    let v1_5_range = VersionReq::parse(">=1.5.0, <1.6.0").unwrap();

    if let ForgeDistribution::Legacy { ref mut fml_libs, .. } = forge_manifest.dist {
        if mc_version == "1.3.2" {
            *fml_libs = Some(ForgeLibrary::fml_libs_1_3());
        } else if v1_4_range.matches(&mc_version_semver) {
            *fml_libs = Some(ForgeLibrary::fml_libs_1_4());
        } else if v1_5_range.matches(&mc_version_semver) {
            *fml_libs = Some(ForgeLibrary::fml_libs_1_5(&mc_version));
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
