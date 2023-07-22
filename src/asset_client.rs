use futures_util::StreamExt;
use semver::Version;
use std::error::Error as StdError;
use std::{io, fs::File, path::Path};
use reqwest::Client;

use crate::Error;
use crate::json::{AssetManifest, VersionManifest, ForgeVersionManifest};

const VERSION_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const FORGE_INDEX_URL: &str = "https://meta.prismlauncher.org/v1/net.minecraftforge/index.json";

pub struct AssetClient {
    client: Client
}

impl AssetClient {
    pub fn new() -> Self {
        AssetClient { client: Client::new() }
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
            .ok_or(Error::new("Minecraft version not found"))?;

        Ok(self.client.get(&version.url)
            .send().await?
            .text().await?)
    }

    pub async fn get_asset_manfiest(&self, url: &str) -> Result<AssetManifest, Box<dyn StdError>> {
        Ok(self.fetch_json(url).await?)
    }

    pub async fn get_forge_manifest_json(&self, forge_version: &str) -> Result<String, Box<dyn StdError>> {
        let index: ForgeVersionManifest = self.fetch_json(FORGE_INDEX_URL).await?;

        index.versions.iter()
            .find(|v| v.version == forge_version)
            .ok_or(Error::new("Forge version not found"))?;

        Ok(self.client.get(FORGE_INDEX_URL.replace("index.json", format!("{forge_version}.json").as_str()))
            .send().await?
            .text().await?)
    }

    pub async fn get_forge_versions(&self, mc_version: &str) -> Result<Vec<ForgeVersion>, Box<dyn StdError>> {
        let index: ForgeVersionManifest = self.fetch_json(FORGE_INDEX_URL).await?;

        let mut versions = index.versions.iter()
            .filter(|v| v.is_for_mc_version(mc_version))
            .map(|f| ForgeVersion::new(&f.version, f.recommended))
            .collect::<Result<Vec<ForgeVersion>, Error>>()?;

        versions.sort_by(|a, b| b.version.cmp(&a.version));

        Ok(versions)
    }
}

pub struct ForgeVersion {
    pub recommended: bool,
    pub version: Version
}

impl ForgeVersion {
    pub fn new(version: &str, recommended: bool) -> Result<Self, Error> {
        Ok(ForgeVersion {
            recommended,
            version: lenient_semver::parse(version)
                .map_err(|e| Error::new(format!("{}", e).as_str()))?
        })
    }
}
