mod env;
mod downloader;
mod json;
mod rules;
pub mod commands;

use downloader::Downloader;
use std::{fs, fs::File, path::Path};
use std::error::Error as StdError;

use env::{get_cache_dir, get_host_os, get_assets_dir};
use json::{GameLibrary, GameLibraryArtifact, GameManifest, AssetManifest};
use rules::RulesMatch;

#[derive(Debug)]
pub struct Error {
    reason: String
}

impl Error {
    pub fn new(reason: &str) -> Self {
        Error{
            reason: String::from(reason)
        }
    }
}

impl StdError for Error { }

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

pub async fn get_game_manifest(mc_version: &str) -> Result<GameManifest, Box<dyn StdError>> {
    let versions_dir = get_cache_dir().join("versions");

    fs::create_dir_all(&versions_dir)?;

    let version_file_path = versions_dir.join(format!("{mc_version}.json"));

    if !version_file_path.exists() {
        let downloader = Downloader::new();
        let game_manifest_json = downloader.get_game_manifest_json(mc_version).await?;

        fs::write(&version_file_path, game_manifest_json)?;
    }

    let version_file = File::open(version_file_path)?;
    let game_manifest: GameManifest = serde_json::from_reader(version_file)?;

    Ok(game_manifest)
}

fn get_client_jar_path(mc_version: &str) -> String {
    format!("com/mojang/minecraft/{mc_version}/minecraft-{mc_version}-client.jar")
}

pub fn get_matched_artifacts(libs: &Vec<GameLibrary>) -> impl Iterator<Item = &GameLibraryArtifact> {
    libs.iter()
        .filter(|lib| lib.rules.is_none() || lib.rules.as_ref().unwrap().matches())
        .flat_map(|lib| {
            let mut result = vec![];

            if let Some(artifact) = &lib.downloads.artifact {
                result.push(artifact);
            }

            if let Some(natives) = &lib.natives {
                let natives_key = natives.get(get_host_os())
                    .expect(format!("os name '{}' not found in lib {} natives", get_host_os(), lib.name).as_str());

                if let Some(classifiers) = &lib.downloads.classifiers {
                    let artifact = classifiers.get(natives_key)
                        .expect(format!("expected key '{}' in lib {} classifiers", natives_key, lib.name).as_str());

                    result.push(artifact);
                } else {
                    panic!("expected 'classifiers' in lib {}", lib.name);
                }
            }

            if result.is_empty() {
                panic!("unhandled download for {}", lib.name);
            }

            return result;
        })
}

pub fn copy_resources(asset_manifest: &AssetManifest, resources_dir: &Path) -> Result<(), Box<dyn StdError>> {
    let assets_dir = get_assets_dir();

    for (path, obj) in asset_manifest.objects.iter() {
        let object_path = assets_dir
            .join("objects")
            .join(&obj.hash[0..2])
            .join(&obj.hash);

        let resource_path = resources_dir.join(path);

        if !resource_path.exists() {
            fs::create_dir_all(resource_path.parent().unwrap())?;
            fs::copy(object_path, resource_path)?;
        }
    }

    Ok(())
}

impl json::GameArgs {
    pub fn matched_args(&self) -> impl Iterator<Item = String> + '_ {
        self.0.iter()
            .filter(|arg| arg.rules.matches())
            .flat_map(|arg| {
                match &arg.value {
                    json::GameArgValue::Single(v) => vec![v.clone()],
                    json::GameArgValue::Many(v) => v.to_vec()
                }
            })
    }
}
