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
use std::{collections::HashMap, io, fs, fs::File, path::Path};
use reqwest::Client;

use crate::{env, Error, ModLoader, ModLoaderName};
use crate::json::{
    AssetManifest, CurseForgeResponse, CurseForgeFile, CurseForgeMod,
    ForgeVersionManifest, ModpackSearch, ModpackManifest, ModpackVersionManifest,
    VersionManifest
};

const VERSION_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const FORGE_INDEX_URL: &str = "https://meta.prismlauncher.org/v1/net.minecraftforge/index.json";
const NEOFORGE_INDEX_URL: &str = "https://meta.prismlauncher.org/v1/net.neoforged/index.json";
const CURSE_MOD_FILES_URL: &str = "https://api.curseforge.com/v1/mods/files";
const CURSE_MODS_URL: &str = "https://api.curseforge.com/v1/mods";
const MODPACKS_CH_URL: &str = "https://api.modpacks.ch/public";
const FTB_PACK_API_URL: &str = "https://api.feed-the-beast.com/v1/modpacks/modpack";

pub struct AssetClient {
    client: Client
}

impl AssetClient {
    pub fn new() -> Self {
        AssetClient { client: Client::new() }
    }

    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        Ok(self.client.get(url)
            .send().await?
            .error_for_status()?
            .json::<T>().await?)
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
        self.fetch_json::<VersionManifest>(VERSION_MANIFEST_URL).await
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
        Ok(self.fetch_json(url).await?)
    }

    pub async fn get_loader_manifest_json(&self, mod_loader: &ModLoader) -> Result<String> {
        let url = match mod_loader.name {
            ModLoaderName::Forge => FORGE_INDEX_URL,
            ModLoaderName::NeoForge => NEOFORGE_INDEX_URL
        };

        let index: ForgeVersionManifest = self.fetch_json(url).await?;

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

        let index: ForgeVersionManifest = self.fetch_json(url).await?;

        let mut versions = index.versions.iter()
            .filter(|v| v.is_for_mc_version(mc_version))
            .map(|f| ModLoaderVersion::new(&f.version, f.recommended))
            .collect::<Result<Vec<ModLoaderVersion>>>()?;

        versions.sort_by(|a, b| b.version.cmp(&a.version));

        Ok(versions)
    }

    pub async fn get_curseforge_file_list(&self, file_ids: &Vec<u64>) -> Result<Vec<CurseForgeFile>> {
        // avoid 400 bad request
        if file_ids.len() == 0 {
            return Ok(Vec::new())
        }

        let response = self.client.post(CURSE_MOD_FILES_URL)
            .header("x-api-key", env::get_curse_api_key())
            .json(&HashMap::from([("fileIds", file_ids)]))
            .send().await?
            .error_for_status()?
            .json::<CurseForgeResponse<CurseForgeFile>>().await?;

        // randomly curseforge returns duplicate entries, remove duplicates
        let mut data = response.data;
        data.dedup_by(|a, b| a.mod_id == b.mod_id);

        Ok(data)
    }

    pub async fn get_curseforge_mods(&self, mod_ids: &Vec<u64>) -> Result<Vec<CurseForgeMod>> {
        // avoid 400 bad request
        if mod_ids.len() == 0 {
            return Ok(Vec::new())
        }

        let response = self.client.post(CURSE_MODS_URL)
            .header("x-api-key", env::get_curse_api_key())
            .json(&HashMap::from([("modIds", mod_ids)]))
            .send().await?
            .error_for_status()?
            .json::<CurseForgeResponse<CurseForgeMod>>().await?;

        Ok(response.data)
    }

    pub async fn get_ftb_modpack_versions(&self, pack_id: u32) -> Result<ModpackManifest> {
        // let url = format!("{MODPACKS_CH_URL}/modpack/{pack_id}");
        let url = format!("{FTB_PACK_API_URL}/{pack_id}");
        let response = self.client.get(url)
            .send().await?
            .error_for_status()?
            .json::<ModpackManifest>().await?;

        Ok(response)
    }

    pub async fn get_ftb_modpack(&self, pack_id: u32, version_id: u32) -> Result<ModpackVersionManifest> {
        // There are some slight inconsistencies between FTB API and modpacks.ch
        // FTB Skies 2 pack has several differences in the "clientonly" flag.
        // When using the FTB API, the files downloaded to build a server pack
        // (excluding clientonly=true) yields a working server, modpacks.ch does not.
        // let url = format!("{MODPACKS_CH_URL}/modpack/{pack_id}/{version_id}");
        let url = format!("{FTB_PACK_API_URL}/{pack_id}/{version_id}");
        let response = self.client.get(url)
            .send().await?
            .error_for_status()?
            .json::<ModpackVersionManifest>().await?;

        Ok(response)
    }

    pub async fn get_curse_modpack_versions(&self, pack_id: u32) -> Result<ModpackManifest> {
        let url = format!("{MODPACKS_CH_URL}/curseforge/{pack_id}");
        let response = self.client.get(url)
            .send().await?
            .error_for_status()?
            .json::<ModpackManifest>().await?;

        Ok(response)
    }

    pub async fn get_curse_modpack(&self, pack_id: u32, version_id: u32) -> Result<ModpackVersionManifest> {
        let url = format!("{MODPACKS_CH_URL}/curseforge/{pack_id}/{version_id}");
        let response = self.client.get(url)
            .send().await?
            .error_for_status()?
            .json::<ModpackVersionManifest>().await?;

        Ok(response)
    }

    /// * `limit` - Search result limit, max 50
    pub async fn search_modpacks(&self, term: &str, limit: u8) -> Result<ModpackSearch> {
        // 50 appears to be max, i.e. setting limit to 99 but response includes "limit: 50"
        let url = format!("{MODPACKS_CH_URL}/modpack/search/{limit}?term={term}");
        let response = self.client.get(url)
            .send().await?
            .error_for_status()?
            .json::<ModpackSearch>().await?;

        Ok(response)
    }
}

impl Default for AssetClient {
    fn default() -> Self {
        Self::new()
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
