use std::error::Error as StdError;
use std::{path::Path, process::Command, collections::HashMap};

use crate::{download_game_files, get_client_jar_path, get_game_manifest, RulesMatch};
use super::{instance::Instance, Progress};
use crate::env::{get_assets_dir, get_libs_dir};
use crate::json::{GameArgValue, GameLibraryDownloads};

pub async fn launch_instance(instance_dir: &Path, progress: &mut dyn Progress) -> Result<(), Box<dyn StdError>> {
    let instance = Instance::load(&instance_dir)?;

    let game_manifest = get_game_manifest(&instance.manifest.mc_version)
        .await?;

    download_game_files(&game_manifest, progress)
        .await?;

    let mut cmd = Command::new("java");
    let mut cmd_args: Vec<String> = vec![];

    if let Some(args) = game_manifest.arguments {
        for arg in args.jvm.0 {
            if !arg.rules.matches() {
                continue;
            }

            match arg.value {
                GameArgValue::Single(v) => cmd_args.push(v),
                GameArgValue::Many(v) => cmd_args.extend(v)
            };
        }

        cmd_args.push(game_manifest.main_class);

        for arg in args.game.0 {
            if !arg.rules.matches() {
                continue;
            }

            match arg.value {
                GameArgValue::Single(v) => cmd_args.push(v),
                GameArgValue::Many(v) => cmd_args.extend(v)
            };
        }
    } else if let Some(args) = game_manifest.minecraft_arguments {
        cmd_args.extend(args.split(' ').map(|v| v.to_string()));
    }

    let mut libs = vec![
        get_client_jar_path(&game_manifest.id)
    ];

    libs.extend(
        game_manifest.libraries.iter().filter_map(|lib| {
            if let Some(rules) = &lib.rules {
                if !rules.matches() {
                    return None;
                }
            }

            match &lib.downloads {
                GameLibraryDownloads::Artifact(x) =>
                    Some(x.artifact.path.clone()),
                GameLibraryDownloads::Classifiers(_) => {
                    // FIXME handle lib with classifiers
                    None
                }
            }
        })
    );

    let classpath = std::env::join_paths(
        libs.iter().map(|p| get_libs_dir().join(p))
    )?.into_string().unwrap();

    let game_dir = instance.game_dir();

    let ctx = HashMap::from([
        ("version_name".into(), instance.manifest.mc_version),
        ("version_type".into(), game_manifest.release_type),
        ("game_directory".into(), game_dir.to_str().unwrap().into()),
        ("assets_root".into(), get_assets_dir().to_str().unwrap().into()),
        ("assets_index_name".into(), "5".into()),
        ("classpath".into(), classpath)
    ]);

    for arg in cmd_args {
        cmd.arg(envsubst::substitute(arg, &ctx)?);
    }

    println!("{:?}", cmd);
    cmd.spawn()?;

    Ok(())
}
