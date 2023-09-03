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

use chrono::{DateTime, serde::ts_seconds, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct AccountManifest {
    pub msa_token: MicrosoftToken,
    pub mc_token: MinecraftToken
}

#[derive(Deserialize, Serialize)]
pub struct MicrosoftToken {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(with = "ts_seconds")]
    pub expires: DateTime<Utc>
}

impl MicrosoftToken {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires
    }
}

#[derive(Deserialize, Serialize)]
pub struct MinecraftToken {
    pub access_token: String,
    #[serde(with = "ts_seconds")]
    pub expires: DateTime<Utc>
}

impl MinecraftToken {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires
    }
}

#[derive(Deserialize, Serialize)]
pub struct MinecraftProfile {
    pub id: String,

    pub name: String,

    pub skins: Vec<MinecraftSkin>,

    pub capes: Vec<MinecraftCape>
}

#[derive(Deserialize, Serialize)]
pub struct MinecraftSkin {
    pub id: String,

    pub state: String,

    pub url: String,

    #[serde(rename(deserialize = "textureKey"))]
    pub texture_key: String,

    pub variant: String
}

#[derive(Deserialize, Serialize)]
pub struct MinecraftCape {
    pub id: String,

    pub state: String,

    pub url: String,

    pub alias: String
}
