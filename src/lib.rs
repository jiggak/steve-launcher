mod env;
mod json;
mod rules;

use env::{get_assets_dir, get_libs_dir};
use json::{
    VersionManifest, VersionManifestEntry,
    GameManifest, GameLibraryDownloads,
    AssetDownload, AssetManifest
};
use rules::match_rules;

use std::{fs::create_dir_all, fs::File, io::copy, path::Path};
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
    fn begin(&mut self, message: &'static str, total: usize);
    fn end(&mut self);

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
    progress.begin("Downloading assets", asset_manifest.objects.len());

    for (_, obj) in asset_manifest.objects.iter() {
        progress.advance(current);
        download_asset(&obj.hash).await?;
        current += 1;
    }

    progress.end();

    let client = game_manifest.downloads.get("client")
        .ok_or(Error::new("Missing 'client' key in downloads object"))?;

    let ver = game_manifest.id;
    let client_path = format!("com/mojang/minecraft/{ver}/minecraft-{ver}-client.jar");
    let mut lib_downloads = vec![
        (client_path.as_str(), client)
    ];

    lib_downloads.extend(
        game_manifest.libraries.iter()
            .filter_map(|lib| {
                if let Some(rules) = &lib.rules {
                    if !match_rules(&rules) {
                        return None;
                    }
                }

                match &lib.downloads {
                    GameLibraryDownloads::Artifact(x) =>
                        Some((x.artifact.path.as_str(), &x.artifact.download)),
                    GameLibraryDownloads::Classifiers(_) => {
                        println!("lib has classifiers");
                        None
                    }
                }
            })
    );

    current = 1;
    progress.begin("Downloading libraries", lib_downloads.len());

    for (path, dl) in lib_downloads {
        progress.advance(current);

        download_library(&path, &dl).await?;

        current += 1;
    }

    progress.end();

    Ok(())
}

async fn download_file(url: &str, file_path: &Path) -> Result<(), Box<dyn StdError>> {
    let mut stream = reqwest::get(url)
        .await?
        .error_for_status()?
        .bytes_stream();

    let mut file = File::create(file_path)?;

    while let Some(item) = stream.next().await {
        copy(&mut item?.as_ref(), &mut file)?;
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

    download_file(&url, &object_file).await
}

async fn download_library(path: &str, download: &AssetDownload) -> Result<(), Box<dyn StdError>> {
    let libs_dir = get_libs_dir();

    let lib_file = libs_dir.join(path);

    // return if lib file already exists
    if lib_file.exists() {
        return Ok(());
    }

    create_dir_all(lib_file.parent().unwrap())?;

    download_file(&download.url, &lib_file).await
}
