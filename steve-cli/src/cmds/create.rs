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

use anyhow::Result;
use dialoguer::{FuzzySelect, Select};
use std::path::Path;

use steve::{AssetClient, Instance, ModLoader, ModLoaderName};

pub async fn create_instance(
    instance_dir: &Path,
    mc_version: Option<String>,
    snapshots: bool,
    mod_loader: Option<String>
) -> Result<()> {
    let mc_version = match mc_version {
        Some(v) => v,
        None => prompt_mc_version(snapshots).await?
    };

    let mod_loader = if let Some(mod_loader_id) = mod_loader {
        if let Ok(mod_loader) = mod_loader_id.parse() {
            Some(mod_loader)
        } else {
            let name = mod_loader_id.parse::<ModLoaderName>()?;
            let version = prompt_loader_version(&mc_version, &name).await?;
            Some(ModLoader { name, version })
        }
    } else {
        None
    };

    Instance::create(instance_dir, &mc_version, mod_loader)
        .await?;

    Ok(())
}

pub async fn prompt_loader_version(mc_version: &str, loader: &ModLoaderName) -> Result<String> {
    let client = AssetClient::new();

    // fetch loader versions for the version of minecraft
    let versions = client.get_loader_versions(mc_version, loader).await?;

    // find the index with recommended flag set to `true`
    let recommend_index = versions.iter()
        .position(|v| v.recommended)
        .unwrap_or(0);

    let selection = Select::with_theme(&super::console_theme())
        .with_prompt(format!("Select {} version (* recommended version)", loader.to_string()))
        .items(&versions)
        .default(recommend_index)
        .interact()?;

    // return the "raw" unparsed version of mod loader from upstream manifest
    Ok(versions[selection].sversion.to_owned())
}

pub async fn prompt_mc_version(snapshots: bool) -> Result<String> {
    let client = AssetClient::new();

    let manifest = client.get_mc_version_manifest().await?;

    let versions: Vec<_> = manifest.versions.into_iter()
        .filter(|v| snapshots || v.release_type == "release")
        .collect();

    let selection = FuzzySelect::with_theme(&super::console_theme())
        .with_prompt("Select Minecraft version")
        .items(&versions)
        .interact()?;

    Ok(versions[selection].id.to_owned())
}
