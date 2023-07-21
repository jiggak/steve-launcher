use std::error::Error as StdError;
use std::{fs, path::Path, process::Command, collections::HashMap};

use crate::{asset_manager, asset_manager::AssetManager, env};
use super::{account::Account, instance::Instance, Progress};


pub async fn launch_instance(instance_dir: &Path, progress: &mut dyn Progress) -> Result<(), Box<dyn StdError>> {
    let instance = Instance::load(&instance_dir)?;
    let account = Account::load_with_tokens().await?;

    let profile = account.fetch_profile().await?;

    let assets = AssetManager::new()?;

    let game_manifest = assets.get_game_manifest(&instance.manifest.mc_version).await?;
    let asset_manifest = assets.get_asset_manfiest(&game_manifest).await?;

    let forge_manifest = match &instance.manifest.forge_version {
        Some(forge_version) => Some(assets.get_forge_manifest(forge_version).await?),
        None => None
    };

    assets.download_assets(&asset_manifest, progress).await?;
    assets.download_libraries(&game_manifest, progress).await?;

    if let Some(forge_manifest) = &forge_manifest {
        assets.download_forge_libraries(forge_manifest, progress).await?;
    }

    let resources_dir = if asset_manifest.is_virtual.unwrap_or(false) {
        Some(assets.virtual_assets_dir(&game_manifest.asset_index.id))
    } else if asset_manifest.map_to_resources.unwrap_or(false) {
        Some(instance.resources_dir())
    } else {
        None
    };

    if let Some(target_dir) = &resources_dir {
        assets.copy_resources(&asset_manifest, target_dir, progress)?;
    }

    assets.extract_natives(&game_manifest, &instance.natives_dir(), progress)?;

    // use java override path from instance manifest, or default to "java" in PATH
    let mut cmd = Command::new(match &instance.manifest.java_path {
        Some(path) => path,
        _ => "java"
    });

    // set current directory for log output
    cmd.current_dir(instance.game_dir());

    fs::create_dir_all(instance.game_dir())?;

    let mut cmd_args: Vec<String> = vec![];

    if let Some(forge_manifest) = &forge_manifest {
        cmd_args.push("-Djava.library.path=${natives_directory}".to_string());
        cmd_args.extend(["-cp".to_string(), "${classpath}".to_string()]);
        cmd_args.push(forge_manifest.main_class.clone());

        if let Some(args) = &forge_manifest.minecraft_arguments {
            cmd_args.extend(args.split(' ').map(|v| v.to_string()));
        } else if let Some(args) = game_manifest.minecraft_arguments {
            cmd_args.extend(args.split(' ').map(|v| v.to_string()));
        }

        if let Some(tweaks) = &forge_manifest.tweakers {
            cmd_args.extend(["--tweakClass".to_string(), tweaks[0].clone()]);
        }

    // newer versions of minecraft
    } else if let Some(args) = game_manifest.arguments {
        cmd_args.extend(args.jvm.matched_args());
        cmd_args.push(game_manifest.main_class);
        cmd_args.extend(args.game.matched_args());

    // older version of minecraft
    } else if let Some(args) = game_manifest.minecraft_arguments {
        // older version don't include JVM args in manifest
        cmd_args.push("-Djava.library.path=${natives_directory}".to_string());
        cmd_args.extend(["-cp".to_string(), "${classpath}".to_string()]);
        cmd_args.push(game_manifest.main_class);
        cmd_args.extend(args.split(' ').map(|v| v.to_string()));
    }

    let mut libs = vec![
        asset_manager::get_client_jar_path(&game_manifest.id)
    ];

    libs.extend(
        game_manifest.libraries.iter()
            .filter(|lib| lib.has_rules_match())
            .filter_map(|lib| lib.downloads.artifact.as_ref())
            .map(|a| a.path.clone())
    );

    if let Some(forge_manifest) = &forge_manifest {
        libs.extend(
            forge_manifest.libraries.iter()
                .map(|lib| lib.asset_path())
        );
    }

    let classpath = std::env::join_paths(
        asset_manager::dedup_libs(&libs)?.iter()
            .map(|p| env::get_libs_dir().join(p))
    )?.into_string().unwrap();

    let game_dir = instance.game_dir();
    let natives_dir = instance.natives_dir();

    let mut arg_ctx = HashMap::from([
        ("version_name", instance.manifest.mc_version),
        ("version_type", game_manifest.release_type),
        ("game_directory", game_dir.to_str().unwrap().into()),
        ("assets_root", env::get_assets_dir().to_str().unwrap().into()),
        ("assets_index_name", game_manifest.asset_index.id),
        ("classpath", classpath),
        ("natives_directory", natives_dir.to_str().unwrap().into()),
        ("user_type", "msa".into()),
        ("clientid", env::get_msa_client_id()),
        ("auth_access_token", account.access_token().into()),
        ("auth_session", format!("token:{token}:{profileId}",
            token = account.access_token(), profileId = profile.id)),
        ("auth_player_name", profile.name),
        ("auth_uuid", profile.id),
        ("launcher_name", env::get_package_name().into()),
        ("launcher_version", env::get_package_version().into()),
        // no idea what this arg does but MC fails to launch unless set to empty json obj
        ("user_properties", "{}".into())
    ]);

    if let Some(path) = &resources_dir {
        arg_ctx.insert("game_assets".into(), path.to_str().unwrap().into());
    }

    for arg in cmd_args {
        cmd.arg(
            shellexpand::env_with_context_no_errors(
                &arg,
                |var:&str| arg_ctx.get(var)
            ).to_string()
        );
    }

    cmd.spawn()?;

    Ok(())
}
