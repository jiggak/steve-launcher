use std::env;
use std::path::PathBuf;

pub fn get_data_dir() -> PathBuf {
    // get base data directory from XDG_DATA_HOME, or ~/.local/share
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
        .expect("MSA_CLIENT_ID env var not found")
}

pub fn get_curse_api_key() -> String {
    env::var("CURSE_API_KEY")
        .expect("CURSE_API_KEY env var not found")
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

