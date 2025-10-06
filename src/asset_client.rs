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
use futures_util::StreamExt;
use semver::Version;
use std::{io, fs, fs::File, path::Path};
use reqwest::{Client, Method, RequestBuilder};

use crate::api_client::ApiClient;
use crate::{ Error, ModLoader, ModLoaderName};
use crate::json::{ AssetManifest, ForgeVersionManifest, VersionManifest };

const VERSION_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const FORGE_INDEX_URL: &str = "https://meta.prismlauncher.org/v1/net.minecraftforge/index.json";
const NEOFORGE_INDEX_URL: &str = "https://meta.prismlauncher.org/v1/net.neoforged/index.json";

pub struct AssetClient {
    client: Client
}

impl AssetClient {
    pub fn new() -> Self {
        AssetClient { client: Client::new() }
    }

    pub async fn download_file(&self,
        url: &str,
        file_path: &Path,
        progress: impl Fn(usize)
    ) -> Result<()> {
        self.download_file_with_length(url, file_path, |_| {}, progress).await
    }

    pub async fn download_file_with_length(&self,
        url: &str,
        file_path: &Path,
        length_cb: impl Fn(usize),
        progress: impl Fn(usize)
    ) -> Result<()> {
        fs::create_dir_all(file_path.parent().unwrap())?;

        let response = self.client.get(url)
            .send().await?
            .error_for_status()?;

        let length = response.content_length();
        if let Some(length) = length {
            length_cb(length as usize)
        }

        let mut stream = response.bytes_stream();

        let mut file = File::create(file_path)?;

        let mut current = 0;

        while let Some(item) = stream.next().await {
            let item = item?;

            current += item.len();
            progress(current);

            io::copy(&mut item.as_ref(), &mut file)?;
        }

        Ok(())
    }

    pub async fn get_mc_version_manifest(&self) -> Result<VersionManifest> {
        self.get(VERSION_MANIFEST_URL).await
    }

    pub async fn get_game_manifest_json(&self, mc_version: &str) -> Result<String> {
        let manifest = self.get_mc_version_manifest().await?;

        let version = manifest.versions.iter()
            .find(|v| v.id == mc_version)
            .ok_or(Error::MinecraftVersionNotFound(mc_version.to_string()))?;

        Ok(self.client.get(&version.url)
            .send().await?
            .text().await?)
    }

    pub async fn get_asset_manfiest(&self, url: &str) -> Result<AssetManifest> {
        Ok(self.get(url).await?)
    }

    pub async fn get_loader_manifest_json(&self, mod_loader: &ModLoader) -> Result<String> {
        let url = match mod_loader.name {
            ModLoaderName::Forge => FORGE_INDEX_URL,
            ModLoaderName::NeoForge => NEOFORGE_INDEX_URL
        };

        let index: ForgeVersionManifest = self.get(url).await?;

        index.versions.iter()
            .find(|v| v.version == mod_loader.version)
            .ok_or(Error::ForgeVersionNotFound(mod_loader.version.clone()))?;

        let file_name = format!("{ver}.json", ver = mod_loader.version);
        Ok(self.client.get(url.replace("index.json", file_name.as_str()))
            .send().await?
            .text().await?)
    }

    pub async fn get_loader_versions(&self,
        mc_version: &str,
        loader: &ModLoaderName
    ) -> Result<Vec<ModLoaderVersion>> {
        let url = match loader {
            ModLoaderName::Forge => FORGE_INDEX_URL,
            ModLoaderName::NeoForge => NEOFORGE_INDEX_URL
        };

        let index: ForgeVersionManifest = self.get(url).await?;

        let mut versions = index.versions.iter()
            .filter(|v| v.is_for_mc_version(mc_version))
            .map(|f| ModLoaderVersion::new(&f.version, f.recommended))
            .collect::<Result<Vec<ModLoaderVersion>>>()?;

        versions.sort_by(|a, b| b.version.cmp(&a.version));

        Ok(versions)
    }
}

impl Default for AssetClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiClient for AssetClient {
    fn request(&self, method: Method, url: &str) -> RequestBuilder {
        self.client.request(method, url)
    }
}

pub struct ModLoaderVersion {
    pub recommended: bool,
    /// Mod loader version as string from the version manifest
    pub sversion: String,
    /// Mod loader version parsed as SemVer
    pub version: Version
}

impl ModLoaderVersion {
    pub fn new(version: &str, recommended: bool) -> Result<Self> {
        Ok(ModLoaderVersion {
            recommended,
            sversion: version.to_string(),
            version: lenient_semver::parse(version)
                .map_err(|_| Error::VersionParse { version: version.into() })
                .with_context(|| format!("Unable to parse SemVer '{version}'"))?
        })
    }
}

impl std::fmt::Display for ModLoaderVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.recommended {
            write!(f, "{ver} *", ver = self.version)
        } else {
            write!(f, "{}", self.version)
        }
    }
}
