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

/**
 * Most of the token code comes from https://github.com/KernelFreeze/minecraft-msa-auth
 */

use chrono::{Duration, OutOfRangeError, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, error::Error as StdError, fs};
use oauth2::{
    AuthUrl, ClientId, DeviceAuthorizationUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
    StandardDeviceAuthorizationResponse, StandardTokenResponse, EmptyExtraTokenFields,
    basic::BasicClient, basic::BasicTokenType, reqwest::async_http_client
};

use crate::env;
use crate::json::{AccountManifest, MicrosoftToken, MinecraftToken, MinecraftProfile};

const MANIFEST_FILE: &str = "account.json";

pub struct Account {
    manifest: AccountManifest
}

pub type LoginCallback = fn(url: &str, code: &str);

impl Account {
    fn write_manifest(&self) -> Result<(), Box<dyn StdError>> {
        let manifest_path = env::get_data_dir().join(MANIFEST_FILE);
        let manifest_json = serde_json::to_string_pretty(&self.manifest)?;
        Ok(fs::write(manifest_path, manifest_json)?)
    }

    pub fn load() -> Result<Self, Box<dyn StdError>> {
        let manifest_path = env::get_data_dir().join(MANIFEST_FILE);
        let json = fs::read_to_string(manifest_path)?;

        Ok(Account {
            manifest: serde_json::from_str::<AccountManifest>(json.as_str())?
        })
    }

    pub async fn load_with_tokens() -> Result<Self, Box<dyn StdError>> {
        let mut account = Self::load()?;

        if account.manifest.msa_token.is_expired() {
            account.manifest.msa_token =
                refresh_token(&account.manifest.msa_token.refresh_token).await?;

            account.write_manifest()?;
        }

        if account.manifest.mc_token.is_expired() {
            account.manifest.mc_token =
                login_token(&account.manifest.msa_token.access_token).await?;

            account.write_manifest()?;
        }

        Ok(account)
    }

    pub async fn login(callback: LoginCallback) -> Result<Account, Box<dyn StdError>> {
        let msa_token = access_token(callback).await?;
        let mc_token = login_token(&msa_token.access_token).await?;

        let account = Account {
            manifest: AccountManifest {
                msa_token, mc_token
            }
        };

        account.write_manifest()?;

        Ok(account)
    }

    pub fn access_token(&self) -> &String {
        &self.manifest.mc_token.access_token
    }

    pub async fn fetch_profile(&self) -> Result<MinecraftProfile, Box<dyn StdError>> {
        get_profile(&self.manifest.mc_token.access_token).await
    }
}

impl MicrosoftToken {
    pub fn from_token_response(
        token_response: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>
    ) -> Result<Self, OutOfRangeError> {
        Ok(Self {
            access_token: token_response.access_token().secret().into(),
            refresh_token: token_response.refresh_token().unwrap().secret().into(),
            expires: Utc::now() + Duration::from_std(token_response.expires_in().unwrap())?
        })
    }
}

fn oauth_client() -> Result<BasicClient, Box<dyn StdError>> {
    let auth_url = AuthUrl::new(
        "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize".to_string()
    )?;

    let token_url = TokenUrl::new(
        "https://login.microsoftonline.com/consumers/oauth2/v2.0/token".to_string()
    )?;

    let device_auth_url = DeviceAuthorizationUrl::new(
        "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode".to_string(),
    )?;

    Ok(BasicClient::new(
        ClientId::new(env::get_msa_client_id().to_string()),
        None,
        auth_url,
        Some(token_url)
    )
    .set_device_authorization_url(device_auth_url))
}

pub async fn access_token(callback: LoginCallback) -> Result<MicrosoftToken, Box<dyn StdError>> {
    let oauth2_client = oauth_client()?;

    let details: StandardDeviceAuthorizationResponse = oauth2_client
        .exchange_device_code()?
        .add_scope(Scope::new("XboxLive.signin".to_string()))
        .add_scope(Scope::new("offline_access".to_string()))
        .request_async(async_http_client)
        .await?;

    callback(
        details.verification_uri(),
        details.user_code().secret()
    );

    let msa_token_result = oauth2_client
        .exchange_device_access_token(&details)
        .request_async(async_http_client, sleep, None)
        .await?;

    Ok(MicrosoftToken::from_token_response(msa_token_result)?)
}

async fn sleep(dur: std::time::Duration) {
    futures_time::task::sleep(dur.into()).await;
}

async fn refresh_token(refresh_token: &str) -> Result<MicrosoftToken, Box<dyn StdError>> {
    let oauth2_client = oauth_client()?;

    let msa_token_result = oauth2_client
        .exchange_refresh_token(&RefreshToken::new(refresh_token.into()))
        .request_async(async_http_client)
        .await?;

    Ok(MicrosoftToken::from_token_response(msa_token_result)?)
}

async fn login_token(msa_access_token: &str) -> Result<MinecraftToken, Box<dyn StdError>> {
    let client = Client::new();

    let xbox_authenticate_json = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": &format!("d={}", msa_access_token)
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let xbox_authenticate_response: XboxAuthResponse = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&xbox_authenticate_json)
        .send().await?
        .error_for_status()?
        .json().await?;

    let xbox_authorize_json = json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbox_authenticate_response.token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let xbox_authorize_response: XboxAuthResponse = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&xbox_authorize_json)
        .send().await?
        .error_for_status()?
        .json().await?;

    let hash = &xbox_authenticate_response.display_claims["xui"][0]["uhs"];
    let token = xbox_authorize_response.token;

    let mc_login_json = json!({
        "identityToken": format!("XBL3.0 x={};{}", hash, token)
    });

    let mc_login_response: MinecraftAuthResponse = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&mc_login_json)
        .send().await?
        .error_for_status()?
        .json().await?;

    Ok(MinecraftToken {
        access_token: mc_login_response.access_token,
        expires: Utc::now() + Duration::seconds(mc_login_response.expires_in.into())
    })
}

async fn get_profile(mc_access_token: &str) -> Result<MinecraftProfile, Box<dyn StdError>> {
    let client = Client::new();

    Ok(client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(mc_access_token)
        .send().await?
        .error_for_status()?
        .json::<MinecraftProfile>().await?)
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct XboxAuthResponse {
    #[serde(rename(deserialize = "IssueInstant"))]
    issue_instant: String,

    #[serde(rename(deserialize = "NotAfter"))]
    not_after: String,

    #[serde(rename(deserialize = "Token"))]
    token: String,

    #[serde(rename(deserialize = "DisplayClaims"))]
    display_claims: HashMap<String, Vec<HashMap<String, String>>>
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct MinecraftAuthResponse {
    username: String,

    roles: Vec<String>,

    access_token: String,

    token_type: String,

    /// Number of seconds until the token expires
    expires_in: u32
}
