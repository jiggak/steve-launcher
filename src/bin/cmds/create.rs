use dialoguer::Select;
use std::{error::Error, path::Path};

use steve::{AssetClient, Instance};

pub async fn create_instance(
    instance_dir: &Path,
    mc_version: &str,
    forge: Option<String>
) -> Result<(), Box<dyn Error>> {
    let forge_version = if let Some(forge_version) = forge {
        if forge_version == "prompt" {
            Some(prompt_forge_version(mc_version).await?)
        } else {
            Some(forge_version)
        }
    } else {
        None
    };

    Instance::create(instance_dir, mc_version, forge_version)
        .await?;

    Ok(())
}

async fn prompt_forge_version(mc_version: &str) -> Result<String, Box<dyn Error>> {
    let client = AssetClient::new();

    let versions = client.get_forge_versions(mc_version).await?;

    let recommend_index = versions.iter()
        .position(|v| v.recommended)
        .unwrap_or(0);

    let items: Vec<_> = versions.iter()
        .map(|v| match v.recommended {
            false => v.version.to_string(),
            true => format!("{ver} *", ver = v.version)
        })
        .collect();

    let selection = Select::with_theme(&super::console_theme())
        .with_prompt("Select Forge version (* recommended version)")
        .items(&items)
        .default(recommend_index)
        .interact()?;

    Ok(versions[selection].version.to_string())
}
