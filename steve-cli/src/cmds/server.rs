/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2024 Josh Kropf <josh@slashdev.ca>
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
use steve::{ModLoader, ModLoaderName, ServerInstance};
use std::path::Path;

use crate::cmds::create::{prompt_loader_version, prompt_mc_version};

pub async fn server_new(
    instance_dir: &Path,
    mc_version: Option<String>,
    mod_loader: Option<String>
) -> Result<()> {
    let mc_version = match mc_version {
        Some(v) => v,
        None => prompt_mc_version(false).await?
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

    ServerInstance::create(instance_dir, &mc_version, mod_loader).await?;

    Ok(())
}

pub async fn server_modpack_ftb(instance_dir: &Path, pack_id: i32) -> Result<()> {
    Ok(println!("Server ftb pack {:?} {pack_id}", instance_dir))
}

pub async fn server_modpack_search(instance_dir: &Path, search: String) -> Result<()> {
    Ok(println!("Server modpack search {:?} {search}", instance_dir))
}

pub async fn server_launch(instance_dir: &Path) -> Result<()> {
    let instance = ServerInstance::load(instance_dir)?;
    let mut result = instance.launch().await?;
    result.wait()?;
    Ok(())
}
