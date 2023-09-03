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
pub struct VersionManifest {
    pub versions: Vec<VersionManifestEntry>
}

#[derive(Deserialize)]
pub struct VersionManifestEntry {
    pub id: String,
    #[serde(rename(deserialize = "type"))]
    pub release_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename(deserialize = "releaseTime"))]
    pub release_time: String,
    pub sha1: String,
    #[serde(rename(deserialize = "complianceLevel"))]
    pub compliance_level: u8
}
