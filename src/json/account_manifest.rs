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
