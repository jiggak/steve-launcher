mod env;
mod json;
mod rules;

use env::{get_assets_dir, get_libs_dir, get_cache_dir};
use json::{
    VersionManifest, VersionManifestEntry,
    GameManifest, GameLibraryDownloads,
    AssetDownload, AssetManifest, InstanceManifest
};
use rules::match_rules;

use std::{fs, io, fs::File, path::Path};
use std::error::Error as StdError;
use futures_util::StreamExt;

const VERSION_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

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

pub trait Progress {
    fn begin(&mut self, message: &'static str, total: usize);
    fn end(&mut self);
    fn advance(&mut self, current: usize);
}

pub async fn get_game_manifest(mc_version: &str) -> Result<GameManifest, Box<dyn StdError>> {
    let versions_dir = get_cache_dir().join("versions");

    fs::create_dir_all(&versions_dir)?;

    let version_file_path = versions_dir.join(format!("{mc_version}.json"));

    if !version_file_path.exists() {
        let manifest: VersionManifest = fetch_json(VERSION_MANIFEST_URL).await?;

        let version = manifest.versions.iter()
            .find(|v| v.id == mc_version)
            .ok_or(Error::new("Version not found"))?;

        let game_manifest_json = fetch_string(version.url.as_str()).await?;

        fs::write(&version_file_path, game_manifest_json)?;
    }

    let version_file = File::open(version_file_path)?;
    let game_manifest: GameManifest = serde_json::from_reader(version_file)?;

    Ok(game_manifest)
}

pub async fn create_instance(instance_dir: &Path, mc_version: &str) -> Result<(), Box<dyn StdError>> {
    // hydrate game manifest cache and validate `mc_version`
    get_game_manifest(mc_version)
        .await?;

    fs::create_dir(instance_dir)?;

    let instance_manifest_path = instance_dir.join("manifest.json");

    let instance_manifest = InstanceManifest {
        mc_version: mc_version.to_string(),
        java_path: None
    };

    let instance_manifest_json = serde_json::to_string_pretty(&instance_manifest)?;
    fs::write(instance_manifest_path, instance_manifest_json)?;

    Ok(())
}

pub async fn launch_instance(instance_dir: &Path, progress: &mut dyn Progress) -> Result<(), Box<dyn StdError>> {
    let instance_manifest = get_instance_manifest(instance_dir)?;
    let game_manifest = get_game_manifest(&instance_manifest.mc_version)
        .await?;

    download_game_files(&game_manifest, progress)
        .await?;

    Ok(())
}

fn get_instance_manifest(instance_dir: &Path) -> Result<InstanceManifest, Box<dyn StdError>> {
    let instance_manifest_path = instance_dir.join("manifest.json");
    let json = fs::read_to_string(instance_manifest_path)?;
    Ok(serde_json::from_str::<InstanceManifest>(json.as_str())?)
}

async fn download_game_files(game_manifest: &GameManifest, progress: &mut dyn Progress) -> Result<(), Box<dyn StdError>> {
    let asset_index_url = game_manifest.asset_index.download.url.as_str();
    let asset_manifest: AssetManifest = fetch_json(asset_index_url)
        .await?;

    let assets_dir = get_assets_dir();
    let indexes_dir = assets_dir.join("indexes");
    fs::create_dir_all(indexes_dir)?;

    let index_file_path = assets_dir
        .join("indexes")
        .join(format!("{ver}.json", ver = game_manifest.asset_index.id));

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

    let client_path = format!("com/mojang/minecraft/{ver}/minecraft-{ver}-client.jar", ver = game_manifest.id);
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

async fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, reqwest::Error> {
    reqwest::get(url)
        .await?
        .json::<T>()
        .await
}

async fn fetch_string(url: &str) -> Result<String, reqwest::Error> {
    reqwest::get(url)
        .await?
        .text()
        .await
}

async fn download_file(url: &str, file_path: &Path) -> Result<(), Box<dyn StdError>> {
    let mut stream = reqwest::get(url)
        .await?
        .error_for_status()?
        .bytes_stream();

    let mut file = File::create(file_path)?;

    while let Some(item) = stream.next().await {
        io::copy(&mut item?.as_ref(), &mut file)?;
    }

    Ok(())
}

async fn download_asset(hash: &str) -> Result<(), Box<dyn StdError>> {
    let hash_prefix = &hash[0..2];
    let assets_dir = get_assets_dir();

    let object_dir = assets_dir
        .join("objects")
        .join(hash_prefix);

    fs::create_dir_all(object_dir)?;

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

    fs::create_dir_all(lib_file.parent().unwrap())?;

    download_file(&download.url, &lib_file).await
}
