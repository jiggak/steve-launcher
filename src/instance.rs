use std::error::Error as StdError;
use std::{collections::HashMap, fs, path::Path, path::PathBuf, process::Child, process::Command};

use crate::{
    account::Account, asset_manager, asset_manager::AssetManager, env,
    json::InstanceManifest, Progress
};

const MANIFEST_FILE: &str = "manifest.json";

pub struct Instance {
    pub manifest: InstanceManifest,

    /// Absolute path of the instance directory
    pub dir: PathBuf
}

impl Instance {
    fn write_manifest(&self) -> Result<(), Box<dyn StdError>> {
        let manifest_path = self.dir.join(MANIFEST_FILE);
        let manifest_json = serde_json::to_string_pretty(&self.manifest)?;
        Ok(fs::write(manifest_path, manifest_json)?)
    }

    fn new(instance_dir: &Path, manifest: InstanceManifest) -> Result<Instance, Box<dyn StdError>> {
        Ok(Instance {
            dir: fs::canonicalize(instance_dir)?,
            manifest
        })
    }

    pub async fn create(instance_dir: &Path, mc_version: &str, forge_version: Option<String>) -> Result<Instance, Box<dyn StdError>> {
        let assets = AssetManager::new()?;

        // validate `mc_version`
        assets.get_game_manifest(mc_version).await?;

        if let Some(forge_version) = &forge_version {
            // validate `forge_version`
            assets.get_forge_manifest(forge_version).await?;
        }

        // create directory to contain instance
        fs::create_dir(instance_dir)?;

        let instance = Instance::new(
            instance_dir,
            InstanceManifest {
                mc_version: mc_version.to_string(),
                game_dir: "minecraft".to_string(),
                java_path: None,
                forge_version: forge_version
            }
        )?;

        // write instance manifest.json file
        instance.write_manifest()?;

        Ok(instance)
    }

    pub fn load(instance_dir: &Path) -> Result<Instance, Box<dyn StdError>> {
        let manifest_path = instance_dir.join(MANIFEST_FILE);
        let json = fs::read_to_string(manifest_path)?;
        let manifest = serde_json::from_str::<InstanceManifest>(json.as_str())?;

        Instance::new(instance_dir, manifest)
    }

    pub fn game_dir(&self) -> PathBuf {
        self.dir.join(&self.manifest.game_dir)
    }

    pub fn resources_dir(&self) -> PathBuf {
        self.game_dir().join("resources")
    }

    pub fn natives_dir(&self) -> PathBuf {
        self.dir.join("natives")
    }

    pub async fn launch(&self, progress: &mut dyn Progress) -> Result<Child, Box<dyn StdError>> {
        let account = Account::load_with_tokens().await?;

        let profile = account.fetch_profile().await?;

        let assets = AssetManager::new()?;

        let game_manifest = assets.get_game_manifest(&self.manifest.mc_version).await?;
        let asset_manifest = assets.get_asset_manfiest(&game_manifest).await?;

        let forge_manifest = match &self.manifest.forge_version {
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
            Some(self.resources_dir())
        } else {
            None
        };

        if let Some(target_dir) = &resources_dir {
            assets.copy_resources(&asset_manifest, target_dir, progress)?;
        }

        assets.extract_natives(&game_manifest, &self.natives_dir(), progress)?;

        // use java override path from instance manifest, or default to "java" in PATH
        let mut cmd = Command::new(match &self.manifest.java_path {
            Some(path) => path,
            _ => "java"
        });

        // set current directory for log output
        cmd.current_dir(self.game_dir());

        fs::create_dir_all(self.game_dir())?;

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

        let game_dir = self.game_dir();
        let natives_dir = self.natives_dir();

        let mut arg_ctx = HashMap::from([
            ("version_name", self.manifest.mc_version.clone()),
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

        Ok(cmd.spawn()?)
    }
}
