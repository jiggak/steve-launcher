use std::error::Error as StdError;
use std::{path::Path, path::PathBuf, process::Command, collections::HashMap};

use crate::asset_manager::AssetManager;
use super::{account::Account, instance::Instance, Progress};
use crate::env::{get_assets_dir, get_libs_dir, get_package_name, get_package_version, get_msa_client_id};

pub async fn launch_instance(instance_dir: &Path, progress: &mut dyn Progress) -> Result<(), Box<dyn StdError>> {
    let instance = Instance::load(&instance_dir)?;
    let account = Account::load_with_tokens().await?;

    let profile = account.fetch_profile().await?;

    let assets = AssetManager::new()?;

    let game_manifest = assets.get_game_manifest(&instance.manifest.mc_version).await?;
    let asset_manifest = assets.get_asset_manfiest(&game_manifest).await?;

    assets.download_assets(&asset_manifest, progress).await?;
    assets.download_libraries(&game_manifest, progress).await?;

    let mut resources_dir: Option<PathBuf> = None;

    if asset_manifest.is_virtual.unwrap_or(false) {
        resources_dir = Some(assets.virtual_assets_dir(&game_manifest.asset_index.id));
    } else if asset_manifest.map_to_resources.unwrap_or(false) {
        resources_dir = Some(instance.resources_dir());
    }

    if let Some(target_dir) = &resources_dir {
        assets.copy_resources(&asset_manifest, target_dir, progress)?;
    }

    assets.extract_natives(&game_manifest, &instance.natives_dir(), progress)?;

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
        AssetManager::get_client_jar_path(&game_manifest.id)
    ];

    libs.extend(
        game_manifest.libraries.iter()
            .filter(|lib| lib.has_rules_match())
            .filter_map(|lib| lib.downloads.artifact.as_ref())
            .map(|a| a.path.clone())
    );

    let classpath = std::env::join_paths(
        libs.iter().map(|p| get_libs_dir().join(p))
    )?.into_string().unwrap();

    let game_dir = instance.game_dir();
    let natives_dir = instance.natives_dir();

    let mut arg_ctx = HashMap::from([
        ("version_name".into(), instance.manifest.mc_version),
        ("version_type".into(), game_manifest.release_type),
        ("game_directory".into(), game_dir.to_str().unwrap().into()),
        ("assets_root".into(), get_assets_dir().to_str().unwrap().into()),
        ("assets_index_name".into(), game_manifest.asset_index.id),
        ("classpath".into(), classpath),
        ("natives_directory".into(), natives_dir.to_str().unwrap().into()),
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

    if let Some(path) = &resources_dir {
        arg_ctx.insert("game_assets".into(), path.to_str().unwrap().into());
    }

    for arg in cmd_args {
        cmd.arg(envsubst::substitute(arg, &arg_ctx)?);
    }

    println!("{:?}", cmd);
    cmd.spawn()?;

    Ok(())
}
