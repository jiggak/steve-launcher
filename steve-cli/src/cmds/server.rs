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
use std::path::Path;


pub fn server_new(
    instance_dir: &Path,
    mc_version: Option<String>,
    mod_loader: Option<String>
) -> Result<()> {
    Ok(println!("Server new {:?} {:?} {:?}", instance_dir, mc_version, mod_loader))
}

pub fn server_modpack_ftb(instance_dir: &Path, pack_id: i32) -> Result<()> {
    Ok(println!("Server ftb pack {:?} {pack_id}", instance_dir))
}

pub fn server_modpack_search(instance_dir: &Path, search: String) -> Result<()> {
    Ok(println!("Server modpack search {:?} {search}", instance_dir))
}

pub fn server_launch(instance_dir: &Path) -> Result<()> {
    Ok(println!("Server launch {:?}", instance_dir))
}
