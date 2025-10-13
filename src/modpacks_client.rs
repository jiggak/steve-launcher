/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2025 Josh Kropf <josh@slashdev.ca>
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

use anyhow::Result;
use reqwest::{Client, Method, RequestBuilder};

use crate::{
    api_client::ApiClient,
    json::{ModpackManifest, ModpackSearch, ModpackVersionManifest}
};

const MODPACKS_CH_URL: &str = "https://api.modpacks.ch/public/";
const FTB_PACK_API_URL: &str = "https://api.feed-the-beast.com/v1/modpacks/modpack/";

pub struct ModpacksClient {
    client: Client
}

impl ModpacksClient {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    pub async fn get_ftb_modpack_versions(&self, pack_id: u32) -> Result<ModpackManifest> {
        // format!("{MODPACKS_CH_URL}/modpack/{pack_id}");
        self.get(&format!("ftb:{pack_id}")).await
    }

    pub async fn get_ftb_modpack(&self, pack_id: u32, version_id: u32) -> Result<ModpackVersionManifest> {
        // There are some slight inconsistencies between FTB API and modpacks.ch
        // FTB Skies 2 pack has several differences in the "clientonly" flag.
        // When using the FTB API, the files downloaded to build a server pack
        // (excluding clientonly=true) yields a working server, modpacks.ch does not.
        // format!("{MODPACKS_CH_URL}/modpack/{pack_id}/{version_id}");
        self.get(&format!("ftb:{pack_id}/{version_id}")).await
    }

    pub async fn get_curse_modpack_versions(&self, pack_id: u32) -> Result<ModpackManifest> {
        self.get(&format!("curseforge/{pack_id}")).await
    }

    pub async fn get_curse_modpack(&self, pack_id: u32, version_id: u32) -> Result<ModpackVersionManifest> {
        self.get(&format!("curseforge/{pack_id}/{version_id}")).await
    }

    /// * `limit` - Search result limit, max 50
    pub async fn search_modpacks(&self, term: &str, limit: u8) -> Result<ModpackSearch> {
        // 50 appears to be max, i.e. setting limit to 99 but response includes "limit: 50"
        self.get(&format!("modpack/search/{limit}?term={term}")).await
    }
}

impl ApiClient for ModpacksClient {
    fn request(&self, method: Method, uri: &str) -> RequestBuilder {
        let url = if uri.starts_with("ftb:") {
            String::from(FTB_PACK_API_URL) + &uri[4..]
        } else {
            String::from(MODPACKS_CH_URL) + uri
        };

        self.client.request(method, url)
    }
}
