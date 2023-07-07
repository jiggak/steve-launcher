use std::error::Error as StdError;
use std::{path::Path, process::Command, collections::HashMap};

use crate::downloader::Downloader;
use crate::{get_client_jar_path, get_game_manifest, get_matched_artifacts, copy_resources};
use super::{account::Account, instance::Instance, Progress};
use crate::env::{get_assets_dir, get_libs_dir, get_package_name, get_package_version, get_msa_client_id};

pub async fn launch_instance(instance_dir: &Path, progress: &mut dyn Progress) -> Result<(), Box<dyn StdError>> {
    let instance = Instance::load(&instance_dir)?;
    let account = Account::load_with_tokens().await?;

    let profile = account.fetch_profile().await?;

    let downloader = Downloader::new();

    let game_manifest = get_game_manifest(&instance.manifest.mc_version).await?;

    // these could all use progress feedback
    // 1. Download game files
    // 2. If required, build resources file structure
    // 3. Extract native jars

    let asset_manifest = downloader.download_game_files(&game_manifest, progress).await?;

    if asset_manifest.is_virtual.unwrap_or(false) {
        copy_resources(&asset_manifest, &get_assets_dir().join("virtual").join(&game_manifest.asset_index.id))?;
    } else if asset_manifest.map_to_resources.unwrap_or(false) {
        copy_resources(&asset_manifest, &instance.resources_dir())?;
    }

    let mut cmd = Command::new(match &instance.manifest.java_path {
        Some(path) => path,
        _ => "java"
    });

    // set current directory for log output
    cmd.current_dir(instance.game_dir());

    let mut cmd_args: Vec<String> = vec![];

    if let Some(args) = game_manifest.arguments {
        cmd_args.extend(args.jvm.matched_args());
        cmd_args.push(game_manifest.main_class);
        cmd_args.extend(args.game.matched_args());
    } else if let Some(args) = game_manifest.minecraft_arguments {
        cmd_args.extend(["-cp".to_string(), "${classpath}".to_string()]);
        cmd_args.push(game_manifest.main_class);
        cmd_args.extend(args.split(' ').map(|v| v.to_string()));
    }

    let mut libs = vec![
        get_client_jar_path(&game_manifest.id)
    ];

    libs.extend(
        get_matched_artifacts(&game_manifest.libraries)
            .map(|a| a.path.clone())
    );

    let classpath = std::env::join_paths(
        libs.iter().map(|p| get_libs_dir().join(p))
    )?.into_string().unwrap();

    let game_dir = instance.game_dir();
    let resources_dir = instance.resources_dir();

    let ctx = HashMap::from([
        ("version_name".into(), instance.manifest.mc_version),
        ("version_type".into(), game_manifest.release_type),
        ("game_directory".into(), game_dir.to_str().unwrap().into()),
        ("assets_root".into(), get_assets_dir().to_str().unwrap().into()),
        ("assets_index_name".into(), game_manifest.asset_index.id),
        ("game_assets".into(), resources_dir.to_str().unwrap().into()),
        ("classpath".into(), classpath),
        // FIXME what should this be? empty value causes various errors
        // instance.dir.join("natives").to_str().unwrap().into()
        ("natives_directory".into(), "/tmp".into()),
        ("user_type".into(), "msa".into()),
        ("clientid".into(), get_msa_client_id()),
        ("auth_access_token".into(), account.access_token().into()),
        ("auth_session".into(), format!("token:{token}:{profileId}",
            token = account.access_token(), profileId = profile.id)),
        ("auth_player_name".into(), profile.name),
        ("auth_uuid".into(), profile.id),
        ("launcher_name".into(), get_package_name().into()),
        ("launcher_version".into(), get_package_version().into())
    ]);

    for arg in cmd_args {
        cmd.arg(envsubst::substitute(arg, &ctx)?);
    }

    println!("{:?}", cmd);
    cmd.spawn()?;

    Ok(())
}
