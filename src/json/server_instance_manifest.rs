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

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::ModLoader;

#[derive(Deserialize, Serialize)]
pub struct ServerInstanceManifest {
    /// Minecraft version
    pub mc_version: String,

    /// Server directory, relative to instance manifest
    pub server_dir: String,

    /// Optional absolute path of Java VM, or use "java" in system path
    pub java_path: Option<String>,

    /// Optional extra JVM arguments
    pub java_args: Option<Vec<String>>,

    /// Optional environment variables
    pub java_env: Option<HashMap<String, String>>,

    /// Optional mod loader
    pub mod_loader: Option<ModLoader>
}
