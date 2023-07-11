use std::error::Error as StdError;
use std::path::Path;

use crate::assets::get_game_manifest;
use super::instance::Instance;

pub async fn create_instance(instance_dir: &Path, mc_version: &str) -> Result<(), Box<dyn StdError>> {
    // hydrate game manifest cache and validate `mc_version`
    get_game_manifest(mc_version)
        .await?;

    Instance::create(instance_dir, mc_version)?;

    Ok(())
}
