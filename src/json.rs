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

mod account_manifest;
mod asset_manifest;
mod curseforge_manifest;
mod forge_manifest;
mod forge_version_manifest;
mod game_manifest;
mod instance_manifest;
mod modpacks_ch;
mod version_manifest;

pub use account_manifest::*;
pub use asset_manifest::*;
pub use curseforge_manifest::*;
pub use forge_manifest::*;
pub use forge_version_manifest::*;
pub use game_manifest::*;
pub use instance_manifest::*;
pub use modpacks_ch::*;
pub use version_manifest::*;

use serde::{Deserialize, Deserializer};

fn empty_string_is_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>
{
    let o: Option<String> = Option::deserialize(deserializer)?;
    Ok(o.filter(|s| !s.is_empty()))
}
