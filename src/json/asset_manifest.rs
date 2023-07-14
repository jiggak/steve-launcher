use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct AssetManifest {
    pub map_to_resources: Option<bool>,
    pub objects: HashMap<String, AssetObject>,
    #[serde(rename(deserialize = "virtual", serialize = "virtual"))]
    pub is_virtual: Option<bool>
}

#[derive(Deserialize, Serialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u32
}
