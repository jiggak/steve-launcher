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

use std::error::Error as StdError;

pub use {
    account::Account,
    asset_client::AssetClient,
    curseforge_zip::CurseForgeZip,
    download_watcher::DownloadWatcher,
    download_watcher::WatcherMessage,
    instance::Instance,
    instance::FileDownload,
    json::ModpackManifest,
    json::ModpackVersion
};


#[derive(Debug)]
pub struct Error {
    reason: String
}

impl Error {
    pub fn new<S: Into<String>>(reason: S) -> Self {
        Error{
            reason: reason.into()
        }
    }
}

impl StdError for Error { }

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

pub trait Progress {
    fn begin(&mut self, message: &'static str, total: usize);
    fn end(&mut self);
    fn advance(&mut self, current: usize);
}
