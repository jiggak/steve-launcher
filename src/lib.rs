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

mod account;
mod asset_client;
mod asset_manager;
mod curseforge_zip;
mod download_watcher;
pub mod env;
mod fs;
mod instance;
mod json;
mod rules;
mod zip;

pub use {
    account::Account,
    asset_client::AssetClient,
    curseforge_zip::CurseForgeZip,
    download_watcher::DownloadWatcher,
    download_watcher::WatcherMessage,
    instance::Instance,
    instance::FileDownload,
    json::ModLoader,
    json::ModLoaderName,
    json::ModpackManifest,
    json::ModpackVersion
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Expected library name '{0}' in format '<group_id>:<artifact_id>:<version>:[classifier]'")]
    InvalidLibraryName(String),
    #[error("Expected library path '{0}' in format '<_>/<version>/<artifact>'")]
    InvalidLibraryPath(String),
    #[error("Missing 'minecraft' target in modpack manifest")]
    MinecraftTargetNotFound,
    #[error("Minecraft version '{0}' not found")]
    MinecraftVersionNotFound(String),
    #[error("Forge version '{0}' not found")]
    ForgeVersionNotFound(String),
    #[error("Unable to parse '{version}' with lenient_semver")]
    VersionParse {
        version: String,
        // FIXME this adds a lifetime requirement I don't want to deal with right now
        // #[source]
        // source: lenient_semver::parser::Error
    },
    #[error("Missing 'net.minecraft' in forge manifest requires list")]
    ForgeRequiresNotFound,
    #[error("CurseForge file results({file_list_len}) do not match mod results({mod_list_len})")]
    CurseFileListMismatch {
        file_list_len: usize,
        mod_list_len: usize
    },
    #[error("Instance directory '{0}' not found or doesn't contain manifest.json file")]
    InstanceNotFound(String),
    #[error("Account credentials not found, run authenticate to save credentials")]
    CredentialNotFound,
    #[error("Invalid mod loader name '{0}'")]
    InvalidModLoaderName(String),
    #[error("Invalid mod loader ID format '{0}'; expected [name]-[version]")]
    InvalidModLoaderId(String)
}

pub trait Progress {
    fn begin(&mut self, message: &'static str, total: usize);
    fn end(&mut self);
    fn advance(&mut self, current: usize);
}
