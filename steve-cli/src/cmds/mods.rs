/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2025 Josh Kropf <josh@slashdev.ca>
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

use std::path::Path;

use anyhow::{anyhow, Result};
use dialoguer::Select;
use steve::{CurseClient, Installer, Instance, ModsManager};

use crate::{cmds::modpack::download_blocked, ProgressBars};

pub async fn mods_status(instance_dir: &Path) -> Result<()> {
    let instance = Instance::load(instance_dir)?;

    // show table of mods (TBD table columns)
    // provide indicator next to mods with new version available
    // interactively update each mod OR shortcut to update all

    let manager = ModsManager::load_curseforge_mods(instance.mods_dir()).await?;
    for m in manager.mods {
        println!("{} {}", m.file_name, m.mod_id);
    }

    Ok(())
}

pub async fn install_mod(instance_dir: &Path, search: &str) -> Result<()> {
    let instance = Instance::load(instance_dir)?;

    let game_dir = instance.game_dir();
    let mc_version = &instance.manifest.mc_version;
    let mod_loader = instance.manifest.mod_loader
        .ok_or(anyhow!("Instance requires mod loader to install mods"))?;

    let loader_name = match mod_loader.name {
        steve::ModLoaderName::Forge => String::from("Forge"),
        steve::ModLoaderName::NeoForge => String::from("NeoForge")
    };

    let client = CurseClient::new();

    let results = client.search_mods(
        mc_version,
        &mod_loader.name.into(),
        search
    ).await?;

    let selection = Select::with_theme(&super::console_theme())
        .with_prompt("Select mod")
        .items(&results)
        .interact()?;

    let selected_mod = &results[selection];

    let files: Vec<_> = selected_mod.latest_files.iter()
        .filter(|f| f.game_versions.contains(&loader_name) && f.game_versions.contains(mc_version))
        .collect();

    if files.len() == 0 {
        return Err(anyhow!("Mod '{}' doesn't have any files matching mod loader '{}'", selected_mod.name, loader_name))
    }

    let file = if files.len() > 1 {
        let selection = Select::with_theme(&super::console_theme())
            .with_prompt("Select file")
            .items(&files)
            .interact()?;

        &files[selection]
    } else {
        &files[0]
    };

    let progress = ProgressBars::new();
    let installer = Installer::new(&game_dir);
    let blocked = installer.install_curseforge_file(file.mod_id, file.file_id, &progress).await?;
    if let Some(blocked) = blocked {
        download_blocked(&installer, blocked)?;
    }

    Ok(())
}
