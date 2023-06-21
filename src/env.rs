use std::env;
use std::path::PathBuf;

pub fn get_data_dir() -> PathBuf {
    let base_data_dir = match env::var("XDG_DATA_HOME") {
        Ok(var) => PathBuf::from(var),
        Err(_) => env::home_dir().unwrap().join(".local").join("share")
    };

    let pkg_name = env::var("CARGO_PKG_NAME")
        .expect("CARGO_PKG_NAME env var not found");

    base_data_dir.join(pkg_name)
}

pub fn get_assets_dir() -> PathBuf {
    let data_dir = get_data_dir();
    data_dir.join("assets")
}
