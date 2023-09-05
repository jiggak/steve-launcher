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

    // fetch forge versions for the version of minecraft
    let versions = client.get_forge_versions(mc_version).await?;

    // find the index with recommended flag set to `true`
    let recommend_index = versions.iter()
        .position(|v| v.recommended)
        .unwrap_or(0);

    let selection = Select::with_theme(&super::console_theme())
        .with_prompt("Select Forge version (* recommended version)")
        .items(&versions)
        .default(recommend_index)
        .interact()?;

    // return the "raw" unparsed version of forge from upstream manifest
    Ok(versions[selection].sversion.to_owned())
}
