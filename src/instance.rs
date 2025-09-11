/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2023 Josh Kropf <josh@slashdev.ca>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use anyhow::{bail, Result};
use std::{fs, path::{Path, PathBuf}, process::Child};

use crate::{
    account::Account,
    asset_manager::{self, get_client_jar_path, make_forge_modded_jar, AssetManager},
    env,
    json::{ForgeDistribution, InstanceManifest, ModLoader},
    launch_cmd::LaunchCommand,
    Error, Progress
};

const MANIFEST_FILE: &str = "manifest.json";

pub struct Instance {
    pub manifest: InstanceManifest,

    /// Absolute path of the instance directory
    pub dir: PathBuf,
}

impl Instance {
    fn write_manifest(&self) -> Result<()> {
        let manifest_path = self.dir.join(MANIFEST_FILE);
        let manifest_json = serde_json::to_string_pretty(&self.manifest)?;
        Ok(fs::write(manifest_path, manifest_json)?)
    }

    fn new(instance_dir: &Path, manifest: InstanceManifest) -> Result<Instance> {
        Ok(Instance {
            dir: fs::canonicalize(instance_dir)?,
            manifest,
        })
    }

    pub fn exists(instance_dir: &Path) -> bool {
        if !instance_dir.exists() || !instance_dir.is_dir() {
            return false;
        }

        instance_dir.join(MANIFEST_FILE).exists()
    }

    pub async fn create(
        instance_dir: &Path,
        mc_version: &str,
        mod_loader: Option<ModLoader>,
    ) -> Result<Instance> {
        let assets = AssetManager::new()?;

        // validate `mc_version`
        assets.get_game_manifest(mc_version).await?;

        if let Some(mod_loader) = &mod_loader {
            // validate `mod_loader`
            assets.get_loader_manifest(mod_loader).await?;
        }

        // create directory to contain instance
        if !instance_dir.exists() {
            fs::create_dir(instance_dir)?;
        }

        let instance = Instance::new(
            instance_dir,
            InstanceManifest {
                mc_version: mc_version.to_string(),
                game_dir: "minecraft".to_string(),
                java_path: None,
                java_args: None,
                java_env: None,
                mod_loader,
                custom_jar: None,
            },
        )?;

        // write instance manifest.json file
        instance.write_manifest()?;

        Ok(instance)
    }

    pub fn load(instance_dir: &Path) -> Result<Instance> {
        let manifest_path = instance_dir.join(MANIFEST_FILE);
        if !manifest_path.exists() {
            bail!(Error::InstanceNotFound(
                instance_dir.to_str().unwrap().to_string()
            ))
        }

        let json = fs::read_to_string(manifest_path)?;
        let manifest = serde_json::from_str::<InstanceManifest>(json.as_str())?;

        Instance::new(instance_dir, manifest)
    }

    pub fn set_mc_version(&mut self, mc_version: String) -> Result<()> {
        self.manifest.mc_version = mc_version;
        self.write_manifest()
    }

    pub fn set_mod_loader(&mut self, mod_loader: Option<ModLoader>) -> Result<()> {
        self.manifest.mod_loader = mod_loader;
        self.write_manifest()
    }

    pub fn game_dir(&self) -> PathBuf {
        self.dir.join(&self.manifest.game_dir)
    }

    pub fn fml_libs_dir(&self) -> PathBuf {
        self.game_dir().join("lib")
    }

    pub fn mods_dir(&self) -> PathBuf {
        self.game_dir().join("mods")
    }

    pub fn resources_dir(&self) -> PathBuf {
        self.game_dir().join("resources")
    }

    pub fn resource_pack_dir(&self) -> PathBuf {
        self.game_dir().join("resourcepacks")
    }

    pub fn shader_pack_dir(&self) -> PathBuf {
        self.game_dir().join("shaderpacks")
    }

    pub fn natives_dir(&self) -> PathBuf {
        self.dir.join("natives")
    }

    pub async fn launch(&self, progress: &mut dyn Progress) -> Result<Child> {
        let account = Account::load_with_tokens().await?;

        let profile = account.fetch_profile().await?;

        let assets = AssetManager::new()?;

        let game_manifest = assets.get_game_manifest(&self.manifest.mc_version).await?;
        let asset_manifest = assets.get_asset_manfiest(&game_manifest).await?;

        let loader_manifest = match &self.manifest.mod_loader {
            Some(mod_loader) => Some(assets.get_loader_manifest(mod_loader).await?),
            None => None,
        };

        assets.download_assets(&asset_manifest, progress).await?;
        assets.download_libraries(&game_manifest, progress).await?;

        if let Some(loader_manifest) = &loader_manifest {
            assets
                .download_loader_libraries(loader_manifest, progress)
                .await?;
        }

        let resources_dir = if asset_manifest.is_virtual.unwrap_or(false) {
            Some(assets.virtual_assets_dir(&game_manifest.asset_index.id))
        } else if asset_manifest.map_to_resources.unwrap_or(false) {
            Some(self.resources_dir())
        } else {
            None
        };

        if let Some(resources_dir) = &resources_dir {
            assets.copy_resources(&asset_manifest, resources_dir, progress)?;
        }

        assets.extract_natives(&game_manifest, &self.natives_dir(), progress)?;

        let mut cmd = LaunchCommand::new(
            &self.game_dir(),
            self.manifest.java_path.as_ref(),
            self.manifest.java_args.as_ref(),
            self.manifest.java_env.as_ref(),
        );

        fs::create_dir_all(self.game_dir())?;

        let mut main_jar: String = get_client_jar_path(&game_manifest.id);

        if let Some(loader_manifest) = &loader_manifest {
            match &loader_manifest.dist {
                // legacy forge distributions required modifying the `minecraft.jar` file
                ForgeDistribution::Legacy { jar_mods, fml_libs } => {
                    main_jar =
                        make_forge_modded_jar(&main_jar, &loader_manifest.version, &jar_mods)?
                            .to_string_lossy()
                            .to_string();

                    // forge will throw an error on startup attempting to download
                    // these libraries (404 not found), unless they already exist
                    if let Some(fml_libs) = fml_libs {
                        super::fs::copy_files(
                            fml_libs
                                .iter()
                                .map(|l| env::get_libs_dir().join(l.asset_path())),
                            self.fml_libs_dir(),
                        )?;
                    }

                    cmd.arg("-Dminecraft.applet.TargetDirectory=${game_directory}");
                    cmd.arg("-Djava.library.path=${natives_directory}");
                    cmd.arg("-Dfml.ignoreInvalidMinecraftCertificates=true");
                    cmd.arg("-Dfml.ignorePatchDiscrepancies=true");
                    cmd.arg("-cp").arg("${classpath}");
                    cmd.arg(game_manifest.main_class);

                    if let Some(args) = game_manifest.minecraft_arguments {
                        cmd.args(args.split(' '));
                    }
                }
                ForgeDistribution::Current {
                    main_class,
                    minecraft_arguments,
                    ..
                } => {
                    cmd.arg("-Djava.library.path=${natives_directory}");
                    cmd.arg("-cp").arg("${classpath}");
                    cmd.arg(main_class);

                    if let Some(args) = minecraft_arguments {
                        cmd.args(args.split(' '));
                    } else if let Some(args) = game_manifest.minecraft_arguments {
                        cmd.args(args.split(' '));
                    }
                }
            }

            if let Some(tweaks) = &loader_manifest.tweakers {
                cmd.arg("--tweakClass").arg(tweaks.first().unwrap());
            }

        // newer versions of minecraft
        } else if let Some(args) = game_manifest.arguments {
            cmd.args(args.jvm.matched_args());
            cmd.arg(game_manifest.main_class);
            cmd.args(args.game.matched_args());

        // older version of minecraft
        } else if let Some(args) = game_manifest.minecraft_arguments {
            // older version don't include JVM args in manifest
            cmd.arg("-Djava.library.path=${natives_directory}");
            cmd.arg("-cp").arg("${classpath}");
            cmd.arg(game_manifest.main_class);
            cmd.args(args.split(' '));
        }

        cmd.arg("--width").arg("854");
        cmd.arg("--height").arg("480");

        if let Some(custom_jar) = &self.manifest.custom_jar {
            main_jar = self.dir.join(custom_jar).to_string_lossy().to_string();
        }

        let mut libs = vec![main_jar];

        libs.extend(
            game_manifest
                .libraries
                .iter()
                .filter(|lib| lib.has_rules_match())
                .filter_map(|lib| lib.downloads.artifact.as_ref())
                .map(|a| a.path.clone()),
        );

        if let Some(loader_manifest) = &loader_manifest {
            if let ForgeDistribution::Current { libraries, .. } = &loader_manifest.dist {
                libs.extend(libraries.iter().map(|lib| lib.asset_path()));
            }
        }

        let classpath = std::env::join_paths(
            asset_manager::dedup_libs(&libs)?
                .iter()
                .map(|p| env::get_libs_dir().join(p)),
        )?;

        let auth_session = format!(
            "token:{token}:{profileId}",
            token = account.access_token(),
            profileId = profile.id
        );

        cmd.arg_ctx("version_name", &self.manifest.mc_version);
        cmd.arg_ctx("version_type", game_manifest.release_type);
        cmd.arg_ctx("game_directory", self.game_dir().to_string_lossy());
        cmd.arg_ctx("assets_root", env::get_assets_dir().to_string_lossy());
        cmd.arg_ctx("assets_index_name", game_manifest.asset_index.id);
        cmd.arg_ctx("classpath", classpath.to_string_lossy());
        cmd.arg_ctx("natives_directory", self.natives_dir().to_string_lossy());
        cmd.arg_ctx("user_type", "msa");
        cmd.arg_ctx("clientid", env::get_msa_client_id());
        cmd.arg_ctx("auth_access_token", account.access_token());
        cmd.arg_ctx("auth_session", auth_session);
        cmd.arg_ctx("auth_player_name", profile.name);
        cmd.arg_ctx("auth_uuid", profile.id);
        cmd.arg_ctx("launcher_name", env::get_package_name());
        cmd.arg_ctx("launcher_version", env::get_package_version());
        // no idea what this arg does but MC fails to launch unless set to empty json obj
        cmd.arg_ctx("user_properties", "{}");

        if let Some(path) = &resources_dir {
            cmd.arg_ctx("game_assets", path.to_string_lossy());
        }

        Ok(cmd.spawn()?)
    }
}
