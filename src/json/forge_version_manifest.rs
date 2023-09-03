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

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ForgeVersionManifest {
    pub versions: Vec<ForgeVersionManifestEntry>
}

#[derive(Deserialize, Clone)]
pub struct ForgeVersionManifestEntry {
    pub recommended: bool,
    #[serde(rename(deserialize = "releaseTime"))]
    pub release_time: String,
    pub requires: Vec<ForgeVersionRequires>,
    pub sha256: String,
    pub version: String
}

impl ForgeVersionManifestEntry {
    pub fn is_for_mc_version(&self, mc_version: &str) -> bool {
        self.requires.iter().any(|r| r.equals == mc_version)
    }
}

#[derive(Deserialize, Clone)]
pub struct ForgeVersionRequires {
    pub equals: String,
    pub uid: String
}
