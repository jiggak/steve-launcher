use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct AssetManifest {
    pub objects: HashMap<String, AssetObject>
}

#[derive(Deserialize, Serialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u32
}
