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
use std::{collections::HashMap, str::FromStr};

use crate::Error;

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

    /// Optional environment variables
    pub java_env: Option<HashMap<String, String>>,

    /// Optional mod loader
    pub mod_loader: Option<ModLoader>,

    /// Optional path to alternate `minecraft.jar`, relative to instance manifest
    pub custom_jar: Option<String>
}

#[derive(Deserialize, Serialize)]
pub enum ModLoaderName {
    #[serde(rename = "forge")]
    Forge,

    #[serde(rename = "neoforge")]
    NeoForge
}

impl FromStr for ModLoaderName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "forge" => Ok(Self::Forge),
            "neoforge" => Ok(Self::NeoForge),
            _ => Err(Error::InvalidModLoaderName(s.into()))
        }
    }
}

impl ToString for ModLoaderName {
    fn to_string(&self) -> String {
        match self {
            Self::Forge => String::from("forge"),
            Self::NeoForge => String::from("neoforge")
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct ModLoader {
    pub name: ModLoaderName,

    pub version: String
}

impl FromStr for ModLoader {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split_once('-')
            .ok_or(Error::InvalidModLoaderId(s.to_string()))?;

        Ok(ModLoader {
            name: parts.0.parse()?,
            version: parts.1.to_string()
        })
    }
}
