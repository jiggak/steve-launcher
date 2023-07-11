use std::{fs, fs::File, path::Path};
use std::error::Error as StdError;

use super::downloader::Downloader;
use super::env::{get_cache_dir, get_assets_dir, get_libs_dir};
use super::json::{GameManifest, AssetManifest};

pub async fn get_game_manifest(mc_version: &str) -> Result<GameManifest, Box<dyn StdError>> {
    let versions_dir = get_cache_dir().join("versions");

    fs::create_dir_all(&versions_dir)?;

    let version_file_path = versions_dir.join(format!("{mc_version}.json"));

    if !version_file_path.exists() {
        let downloader = Downloader::new();
        let game_manifest_json = downloader.get_game_manifest_json(mc_version).await?;

        fs::write(&version_file_path, game_manifest_json)?;
    }

    let version_file = File::open(version_file_path)?;
    let game_manifest: GameManifest = serde_json::from_reader(version_file)?;

    Ok(game_manifest)
}

pub async fn get_asset_manfiest(game_manifest: &GameManifest) -> Result<AssetManifest, Box<dyn StdError>> {
    let index_file_path = get_assets_dir()
        .join("indexes")
        .join(format!("{ver}.json", ver = game_manifest.asset_index.id));

    fs::create_dir_all(index_file_path.parent().unwrap())?;

    if index_file_path.exists() {
        let index_file = File::open(&index_file_path)?;
        serde_json::from_reader(index_file)?
    }

    let downloader = Downloader::new();

    let asset_index_url = game_manifest.asset_index.download.url.as_str();
    let asset_manifest = downloader.get_asset_manfiest(asset_index_url).await?;

    let index_file = File::create(index_file_path)?;
    serde_json::to_writer(index_file, &asset_manifest)?;

    Ok(asset_manifest)
}

pub fn get_client_jar_path(mc_version: &str) -> String {
    format!("com/mojang/minecraft/{mc_version}/minecraft-{mc_version}-client.jar")
}

pub fn copy_resources(asset_manifest: &AssetManifest, resources_dir: &Path) -> Result<(), Box<dyn StdError>> {
    let assets_dir = get_assets_dir();

    for (path, obj) in asset_manifest.objects.iter() {
        let object_path = assets_dir
            .join("objects")
            .join(&obj.hash[0..2])
            .join(&obj.hash);

        let resource_path = resources_dir.join(path);

        if !resource_path.exists() {
            fs::create_dir_all(resource_path.parent().unwrap())?;
            fs::copy(object_path, resource_path)?;
        }
    }

    Ok(())
}

pub fn extract_natives(game_manifest: &GameManifest, target_dir: &Path) -> Result<(), Box<dyn StdError>> {
    let native_libs = game_manifest.libraries.iter()
        .filter(|lib| lib.has_rules_match())
        .filter_map(|lib| lib.natives_artifact());

    for lib in native_libs {
        let lib_file = get_libs_dir().join(&lib.path);
        zip_extract::extract(File::open(lib_file)?, target_dir, false)?;
    }

    Ok(())
}
