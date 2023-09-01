mod account_manifest;
mod asset_manifest;
mod curseforge_manifest;
mod forge_manifest;
mod forge_version_manifest;
mod game_manifest;
mod instance_manifest;
mod modpacks_ch;
mod version_manifest;

pub use account_manifest::*;
pub use asset_manifest::*;
pub use curseforge_manifest::*;
pub use forge_manifest::*;
pub use forge_version_manifest::*;
pub use game_manifest::*;
pub use instance_manifest::*;
pub use modpacks_ch::*;
pub use version_manifest::*;

use serde::{Deserialize, Deserializer};

fn empty_string_is_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>
{
    let o: Option<String> = Option::deserialize(deserializer)?;
    Ok(o.filter(|s| !s.is_empty()))
}

fn int_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
    // D::Error: serde::de::Error
{
    // let result = String::deserialize(deserializer);
    // if result.is_ok() {
    //     return Ok(result.unwrap())
    // }

    // let result = i32::deserialize(deserializer);
    // if result.is_ok() {
    //     Ok(result.unwrap().to_string())
    // } else {
    //     Err(result.unwrap_err())
    // }

    use serde_json::Value;

    let val = Value::deserialize(deserializer)?;
    match val {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        // what no worky?
        // _ => Err(D::Error::custom("overflow"))
        _ => Ok(String::from("Foobar"))
    }
}
