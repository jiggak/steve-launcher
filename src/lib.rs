mod account;
mod asset_client;
mod asset_manager;
mod curseforge_zip;
mod download_watcher;
mod env;
mod instance;
mod json;
mod rules;

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
