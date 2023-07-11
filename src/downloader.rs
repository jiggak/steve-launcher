use futures_util::StreamExt;
use std::error::Error as StdError;
use std::{io, fs::File, path::Path};
use reqwest::Client;

use crate::Error;
use crate::json::{AssetManifest, VersionManifest};

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

    pub async fn download_file(&self, url: &str, file_path: &Path) -> Result<(), Box<dyn StdError>> {
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
}
