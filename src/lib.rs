mod env;
mod json;

use env::get_assets_dir;
use json::{VersionManifest, VersionManifestEntry, GameManifest, AssetDownload, AssetManifest};

use std::{fs::create_dir_all, fs::File, io::copy};
use std::error::Error as StdError;
use futures_util::StreamExt;

#[derive(Debug)]
pub struct Error {
    reason: String
}

impl Error {
    pub fn new(reason: &str) -> Self {
        Error{
            reason: String::from(reason)
        }
    }
}

impl StdError for Error { }

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

async fn get_version_manifest() -> Result<VersionManifest, reqwest::Error> {
    reqwest::get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .await?
        .json::<VersionManifest>()
        .await
}

async fn get_game_manifest(version: &VersionManifestEntry) -> Result<GameManifest, reqwest::Error> {
    reqwest::get(version.url.as_str())
        .await?
        .json::<GameManifest>()
        .await
}

async fn get_asset_manifest(asset: &AssetDownload) -> Result<AssetManifest, reqwest::Error> {
    reqwest::get(asset.url.as_str())
        .await?
        .json::<AssetManifest>()
        .await
}

pub trait Progress {
    fn begin(&mut self, total: usize);

    fn advance(&mut self, current: usize);
}

pub async fn create_instance(dir: &str, mc_version: &str, progress: &mut dyn Progress) -> Result<(), Box<dyn StdError>> {
    let manifest = get_version_manifest()
        .await?;

    let version = manifest.versions.iter()
        .find(|v| v.id == mc_version)
        .ok_or(Error::new("Version not found"))?;

    let game_manifest = get_game_manifest(version)
        .await?;

    let asset_manifest = get_asset_manifest(&game_manifest.asset_index.download)
        .await?;

    let assets_dir = get_assets_dir();
    let indexes_dir = assets_dir.join("indexes");
    create_dir_all(indexes_dir)?;

    let index_file_path = assets_dir
        .join("indexes")
        .join(game_manifest.asset_index.id + ".json");

    let index_file = File::create(index_file_path)?;
    serde_json::to_writer(index_file, &asset_manifest)?;

    let mut current: usize = 1;

    progress.begin(asset_manifest.objects.len());

    for (_, obj) in asset_manifest.objects.iter() {
        progress.advance(current);
        download_asset(&obj.hash).await?;
        current += 1;
    }

    Ok(())
}

async fn download_asset(hash: &str) -> Result<(), Box<dyn StdError>> {
    let hash_prefix = &hash[0..2];
    let assets_dir = get_assets_dir();

    let object_dir = assets_dir
        .join("objects")
        .join(hash_prefix);

    create_dir_all(object_dir)?;

    let object_file = assets_dir
        .join("objects")
        .join(hash_prefix)
        .join(hash);

    // return if object file already exists
    if object_file.exists() {
        return Ok(());
    }

    let url = format!("https://resources.download.minecraft.net/{hash_prefix}/{hash}");

    let mut stream = reqwest::get(url)
        .await?
        .error_for_status()?
        .bytes_stream();

    let mut file = File::create(object_file)?;

    while let Some(item) = stream.next().await {
        copy(&mut item?.as_ref(), &mut file)?;
    }

    Ok(())
}
