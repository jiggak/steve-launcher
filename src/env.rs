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

use std::env;
use std::path::PathBuf;

pub fn set_data_dir(path: &str) {
    env::set_var("STEVE_DATA_HOME", path)
}

pub fn get_data_dir() -> PathBuf {
    // get data directory resolve order:
    // $STEVE_DATA_HOME, $XDG_DATA_HOME/steve, $HOME/share/steve
    match env::var("STEVE_DATA_HOME") {
        Ok(var) => PathBuf::from(var),
        Err(_) => {
            let base_data_dir = match env::var("XDG_DATA_HOME") {
                Ok(var) => PathBuf::from(var),
                Err(_) => {
                    let home_dir = env::var("HOME")
                        .expect("HOME env var not found");

                    PathBuf::from(home_dir).join(".local").join("share")
                }
            };

            base_data_dir.join(get_package_name())
        }
    }
}

pub fn get_assets_dir() -> PathBuf {
    get_data_dir().join("assets")
}

pub fn get_libs_dir() -> PathBuf {
    get_data_dir().join("libraries")
}

pub fn get_cache_dir() -> PathBuf {
    get_data_dir().join("cache")
}

pub fn get_host_os() -> &'static str {
    match env::consts::OS {
        // mojang json files uses "osx" instead of "macos" for os name
        "macos" => "osx",
        os => os
    }
}

pub fn get_package_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

pub fn get_package_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn get_msa_client_id() -> String {
    env::var("MSA_CLIENT_ID")
        .map_or(env!("MSA_CLIENT_ID").to_string(), |val| val)
}

pub fn get_curse_api_key() -> String {
    env::var("CURSE_API_KEY")
        .map_or(env!("CURSE_API_KEY").to_string(), |val| val)
}

pub fn get_downloads_dir() -> PathBuf {
    match env::var("XDG_DOWNLOAD_DIR") {
        Ok(var) => PathBuf::from(var),
        Err(_) => {
            let home_dir = env::var("HOME")
                .expect("HOME env var not found");

            PathBuf::from(home_dir).join("Downloads")
        }
    }
}

pub fn get_user_name() -> String {
    env::var("USER")
        .expect("USER env var not found")
}
