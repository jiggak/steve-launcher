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
use serde_json::{json, Value};
use url::form_urlencoded;

use crate::{api_client::ApiClient, env};
use crate::json::{
    CurseForgeFile, CurseForgeFingerprintMatches, CurseForgeMod,
    CurseForgeResponse, CurseForgeResponseWithPaging, ModLoaderType,
    ModSearchSortField
};

const MC_GAME_ID: u32 = 432;
const CURSE_API_URL: &str = "https://api.curseforge.com/v1/";

pub struct CurseClient {
    client: Client
}

impl CurseClient {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    pub async fn get_files(&self, file_ids: &Vec<u32>) -> Result<Vec<CurseForgeFile>> {
        // avoid 400 bad request
        if file_ids.len() == 0 {
            return Ok(Vec::new())
        }

        let response: CurseForgeResponse<Vec<CurseForgeFile>> =
            self.post("mods/files", &json!({"fileIds": file_ids}))
            .await?;

        // randomly curseforge returns duplicate entries, remove duplicates
        let mut data = response.data;
        data.dedup_by(|a, b| a.mod_id == b.mod_id);

        Ok(data)
    }

    pub async fn get_mods(&self, mod_ids: &Vec<u32>) -> Result<Vec<CurseForgeMod>> {
        // avoid 400 bad request
        if mod_ids.len() == 0 {
            return Ok(Vec::new())
        }

        let response: CurseForgeResponse<_> =
            self.post("mods", &json!({"modIds": mod_ids}))
            .await?;

        Ok(response.data)
    }

    pub async fn get_fingerprints(&self, fingerprints: &Vec<u32>) -> Result<CurseForgeFingerprintMatches> {
        let response: CurseForgeResponse<_> =
            self.post(
                &format!("fingerprints/{MC_GAME_ID}"),
                &json!({"fingerprints": fingerprints})
            )
            .await?;

        Ok(response.data)
    }

    pub async fn search_mods(&self,
        mc_version: &str,
        mod_loader: ModLoaderType,
        search: &str
    ) -> Result<Vec<CurseForgeMod>> {
        let params = json!({
            "gameId": MC_GAME_ID,
            "classId": 6, // "6" is "Mods" category
            "gameVersion": mc_version,
            "modLoaderType": mod_loader,
            "searchFilter": search,
            "sortField": ModSearchSortField::Popularity,
            "sortOrder": "desc"
        });

        let query = to_query_string(params);
        let response: CurseForgeResponseWithPaging<_> =
            self.get(&format!("mods/search?{query}"))
            .await?;

        Ok(response.data)
    }
}

fn to_query_string(params: Value) -> String {
    let params: Vec<_> = params.as_object().unwrap().iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();

    form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params)
        .finish()
}

impl ApiClient for CurseClient {
    fn request(&self, method: Method, uri: &str) -> RequestBuilder {
        let url = String::from(CURSE_API_URL) + uri;
        self.client.request(method, url)
            .header("x-api-key", env::get_curse_api_key())
    }
}
