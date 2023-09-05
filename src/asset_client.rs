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

use futures_util::StreamExt;
use semver::Version;
use std::error::Error as StdError;
use std::{collections::HashMap, io, fs, fs::File, path::Path};
use reqwest::Client;

use crate::{env, Error};
use crate::json::{
    AssetManifest, CurseForgeResponse, CurseForgeFile, CurseForgeMod,
    ForgeVersionManifest, ModpackSearch, ModpackManifest, ModpackVersionManifest,
    VersionManifest
};

const VERSION_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const FORGE_INDEX_URL: &str = "https://meta.prismlauncher.org/v1/net.minecraftforge/index.json";
const CURSE_MOD_FILES_URL: &str = "https://api.curseforge.com/v1/mods/files";
const CURSE_MODS_URL: &str = "https://api.curseforge.com/v1/mods";
const MODPACKS_CH_URL: &str = "https://api.modpacks.ch/public";

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
        fs::create_dir_all(file_path.parent().unwrap())?;

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
            .ok_or(Error::new(format!("Forge version '{forge_version}' not found")))?;

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

    pub async fn get_curseforge_file_list(&self, file_ids: &Vec<u64>) -> Result<Vec<CurseForgeFile>, Box<dyn StdError>> {
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

    pub async fn get_curseforge_mods(&self, mod_ids: &Vec<u64>) -> Result<Vec<CurseForgeMod>, Box<dyn StdError>> {
        let response = self.client.post(CURSE_MODS_URL)
            .header("x-api-key", env::get_curse_api_key())
            .json(&HashMap::from([("modIds", mod_ids)]))
            .send().await?
            .error_for_status()?
            .json::<CurseForgeResponse<CurseForgeMod>>().await?;

        Ok(response.data)
    }

    pub async fn get_ftb_modpack_versions(&self, pack_id: u32) -> Result<ModpackManifest, Box<dyn StdError>> {
        let response = self.client.get(format!("{MODPACKS_CH_URL}/modpack/{pack_id}"))
            .send().await?
            .error_for_status()?
            .json::<ModpackManifest>().await?;

        Ok(response)
    }

    pub async fn get_ftb_modpack(&self, pack_id: u32, version_id: u32) -> Result<ModpackVersionManifest, Box<dyn StdError>> {
        let response = self.client.get(format!("{MODPACKS_CH_URL}/modpack/{pack_id}/{version_id}"))
            .send().await?
            .error_for_status()?
            .json::<ModpackVersionManifest>().await?;

        Ok(response)
    }

    pub async fn get_curse_modpack_versions(&self, pack_id: u32) -> Result<ModpackManifest, Box<dyn StdError>> {
        let response = self.client.get(format!("{MODPACKS_CH_URL}/curseforge/{pack_id}"))
            .send().await?
            .error_for_status()?
            .json::<ModpackManifest>().await?;

        Ok(response)
    }

    pub async fn get_curse_modpack(&self, pack_id: u32, version_id: u32) -> Result<ModpackVersionManifest, Box<dyn StdError>> {
        let response = self.client.get(format!("{MODPACKS_CH_URL}/curseforge/{pack_id}/{version_id}"))
            .send().await?
            .error_for_status()?
            .json::<ModpackVersionManifest>().await?;

        Ok(response)
    }

    /// * `limit` - Search result limit, max 50
    pub async fn search_modpacks(&self, term: &str, limit: u8) -> Result<ModpackSearch, Box<dyn StdError>> {
        // 50 appears to be max, i.e. setting limit to 99 but response includes "limit: 50"
        let response = self.client.get(format!("{MODPACKS_CH_URL}/modpack/search/{limit}?term={term}"))
            .send().await?
            .error_for_status()?
            .json::<ModpackSearch>().await?;

        Ok(response)
    }
}

pub struct ForgeVersion {
    pub recommended: bool,
    /// Forge version as string from the version manifest
    pub sversion: String,
    /// Forge version parsed as SemVer
    pub version: Version
}

impl ForgeVersion {
    pub fn new(version: &str, recommended: bool) -> Result<Self, Error> {
        Ok(ForgeVersion {
            recommended,
            sversion: version.to_string(),
            version: lenient_semver::parse(version)
                .map_err(|e| Error::new(format!("{}", e)))?
        })
    }
}

impl ToString for ForgeVersion {
    fn to_string(&self) -> String {
        if self.recommended {
            format!("{ver} *", ver = self.version)
        } else {
            self.version.to_string()
        }
    }
}
