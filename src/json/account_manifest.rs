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
