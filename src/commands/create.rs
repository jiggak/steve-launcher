use semver::Version;
use std::{error::Error as StdError, path::Path};

use crate::{asset_client::AssetClient, asset_manager::AssetManager, Error};
use super::instance::Instance;

pub async fn create_instance(instance_dir: &Path, mc_version: &str, forge_version: Option<String>) -> Result<(), Box<dyn StdError>> {
    let assets = AssetManager::new()?;

    // validate `mc_version`
    assets.get_game_manifest(mc_version).await?;

    if let Some(forge_version) = &forge_version {
        // validate `forge_version`
        assets.get_forge_manifest(forge_version).await?;
    }

    Instance::create(instance_dir, mc_version, forge_version)?;

    Ok(())
}

pub async fn get_forge_versions(mc_version: &String) -> Result<Vec<ForgeVersion>, Box<dyn StdError>> {
    let client = AssetClient::new();

    let result = client.get_forge_versions(&mc_version).await?;
    let mut versions = result.iter()
        .map(|f| ForgeVersion::new(&f.version, f.recommended))
        .collect::<Result<Vec<ForgeVersion>, Error>>()?;

    versions.sort_by(|a, b| b.version.cmp(&a.version));

    Ok(versions)
}

pub struct ForgeVersion {
    pub recommended: bool,
    pub version: Version
}

impl ForgeVersion {
    pub fn new(version: &str, recommended: bool) -> Result<Self, Error> {
        Ok(ForgeVersion {
            recommended,
            version: lenient_semver::parse(version)
                .map_err(|e| Error::new(format!("{}", e).as_str()))?
        })
    }
}
