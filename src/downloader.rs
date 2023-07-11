use futures_util::StreamExt;
use std::error::Error as StdError;
use std::{fs, io, fs::File, path::Path};
use reqwest::Client;

use crate::assets::{get_client_jar_path};
use crate::{Error, commands::Progress};
use crate::env::{get_assets_dir, get_libs_dir};
use crate::json::{AssetDownload, AssetManifest, GameManifest, VersionManifest};

const VERSION_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

pub struct Downloader {
    client: Client
}

impl Downloader {
    pub fn new() -> Self {
        Downloader { client: Client::new() }
    }

    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, reqwest::Error> {
        self.client.get(url)
            .send().await?
            .json::<T>().await
    }

    async fn download_file(&self, url: &str, file_path: &Path) -> Result<(), Box<dyn StdError>> {
        let mut stream = self.client.get(url)
            .send().await?
            .error_for_status()?
            .bytes_stream();

        let mut file = File::create(file_path)?;

        while let Some(item) = stream.next().await {
            io::copy(&mut item?.as_ref(), &mut file)?;
        }

        Ok(())
    }

    async fn download_asset(&self, hash: &str) -> Result<(), Box<dyn StdError>> {
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

        self.download_file(&url, &object_file).await
    }

    async fn download_library(&self, path: &str, download: &AssetDownload) -> Result<(), Box<dyn StdError>> {
        let libs_dir = get_libs_dir();

        let lib_file = libs_dir.join(path);

        // return if lib file already exists
        if lib_file.exists() {
            return Ok(());
        }

        fs::create_dir_all(lib_file.parent().unwrap())?;

        self.download_file(&download.url, &lib_file).await
    }

    pub async fn get_game_manifest_json(&self, mc_version: &str) -> Result<String, Box<dyn StdError>> {
        let manifest: VersionManifest = self.fetch_json(VERSION_MANIFEST_URL).await?;

        let version = manifest.versions.iter()
            .find(|v| v.id == mc_version)
            .ok_or(Error::new("Version not found"))?;

        Ok(self.client.get(&version.url)
            .send().await?
            .text().await?)
    }

    pub async fn get_asset_manfiest(&self, url: &str) -> Result<AssetManifest, Box<dyn StdError>> {
        Ok(self.fetch_json(url).await?)
    }

    pub async fn download_game_files(&self,
        game_manifest: &GameManifest,
        asset_manifest: &AssetManifest,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        progress.begin("Downloading assets", asset_manifest.objects.len());

        for (i, (_, obj)) in asset_manifest.objects.iter().enumerate() {
            progress.advance(i + 1);
            self.download_asset(&obj.hash).await?;
        }

        progress.end();

        let client = game_manifest.downloads.get("client")
            .ok_or(Error::new("Missing 'client' key in downloads object"))?;

        let client_path = get_client_jar_path(&game_manifest.id);
        let mut lib_downloads: Vec<(&str, &AssetDownload)> = vec![
            (client_path.as_str(), client)
        ];

        lib_downloads.extend(
            game_manifest.libraries.iter()
                .filter(|lib| lib.has_rules_match())
                .flat_map(|lib| {
                    // FIXME I think this can be simplified
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
                .map(|a| (a.path.as_str(), &a.download))
        );

        progress.begin("Downloading libraries", lib_downloads.len());

        for (i, (path, dl)) in lib_downloads.iter().enumerate() {
            progress.advance(i + 1);

            self.download_library(&path, &dl).await?;
        }

        progress.end();

        Ok(())
    }
}
