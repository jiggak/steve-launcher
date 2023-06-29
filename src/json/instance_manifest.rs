use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct InstanceManifest {
    pub mc_version: String,
    pub game_dir: String,
    pub java_path: Option<String>
}
