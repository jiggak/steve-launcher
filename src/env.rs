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

pub fn get_package_name() -> String {
    env::var("CARGO_PKG_NAME")
        .expect("CARGO_PKG_NAME env var not found")
}

pub fn get_package_version() -> String {
    env::var("CARGO_PKG_VERSION")
        .expect("CARGO_PKG_VERSION env var not found")
}

pub fn get_azure_client_id() -> String {
    env::var("AZURE_CLIENT_ID")
        .expect("AZURE_CLIENT_ID env var not found")
}
