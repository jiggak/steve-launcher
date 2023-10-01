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

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct InstanceManifest {
    /// Minecraft version
    pub mc_version: String,

    /// Minecraft directory, relative to instance manifest
    pub game_dir: String,

    /// Optional absolute path of Java VM, or use "java" in system path
    pub java_path: Option<String>,

    /// Optional extra JVM arguments
    pub java_args: Option<Vec<String>>,

    /// Optional Forge version
    pub forge_version: Option<String>,

    /// Optional path to alternate `minecraft.jar`, relative to instance manifest
    pub custom_jar: Option<String>
}
