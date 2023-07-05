use std::error::Error as StdError;
use std::{path::Path, process::Command, collections::HashMap};

use crate::{download_game_files, get_client_jar_path, get_game_manifest, get_matched_artifacts};
use super::{account::Account, instance::Instance, Progress};
use crate::env::{get_assets_dir, get_libs_dir, get_package_name, get_package_version, get_azure_client_id};

pub async fn launch_instance(instance_dir: &Path, progress: &mut dyn Progress) -> Result<(), Box<dyn StdError>> {
    let instance = Instance::load(&instance_dir)?;
    let account = Account::load_with_tokens().await?;

    let game_manifest = get_game_manifest(&instance.manifest.mc_version)
        .await?;

    download_game_files(&game_manifest, progress)
        .await?;

    let mut cmd = Command::new("java");
    let mut cmd_args: Vec<String> = vec![];

    if let Some(args) = game_manifest.arguments {
        cmd_args.extend(args.jvm.matched_args());

        cmd_args.push(game_manifest.main_class);

        cmd_args.extend(args.game.matched_args());
    } else if let Some(args) = game_manifest.minecraft_arguments {
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

    let ctx = HashMap::from([
        ("version_name".into(), instance.manifest.mc_version),
        ("version_type".into(), game_manifest.release_type),
        ("game_directory".into(), game_dir.to_str().unwrap().into()),
        ("assets_root".into(), get_assets_dir().to_str().unwrap().into()),
        ("assets_index_name".into(), game_manifest.asset_index.id),
        ("classpath".into(), classpath),
        ("natives_directory".into(), "".into()),
        ("user_type".into(), "msa".into()),
        ("clientid".into(), get_azure_client_id()),
        ("auth_access_token".into(), account.access_token().into()),
        // TODO fill these in from profile
        // ("auth_player_name".into(), player_name),
        // ("auth_uuid".into(), profile_id),
        ("launcher_name".into(), get_package_name()),
        ("launcher_version".into(), get_package_version())
    ]);

    for arg in cmd_args {
        cmd.arg(envsubst::substitute(arg, &ctx)?);
    }

    println!("{:?}", cmd);
    cmd.spawn()?;

    Ok(())
}