/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2024 Josh Kropf <josh@slashdev.ca>
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

use std::{fs, path::{Path, PathBuf}, process::Child};

use anyhow::{bail, Result};

use crate::{
    asset_manager::AssetManager,
    json::ServerInstanceManifest,
    launch_cmd::LaunchCommand,
    BeginProgress, Error, ModLoader, ModLoaderName
};

const MANIFEST_FILE: &str = "manifest.json";

pub struct ServerInstance {
    pub manifest: ServerInstanceManifest,

    /// Absolute path of the instance directory
    pub dir: PathBuf
}

impl ServerInstance {
    fn write_manifest(&self) -> Result<()> {
        let manifest_path = self.dir.join(MANIFEST_FILE);
        let manifest_json = serde_json::to_string_pretty(&self.manifest)?;
        Ok(fs::write(manifest_path, manifest_json)?)
    }

    fn new(instance_dir: &Path, manifest: ServerInstanceManifest) -> Result<Self> {
        Ok(Self {
            dir: fs::canonicalize(instance_dir)?,
            manifest
        })
    }

    pub fn server_dir(&self) -> PathBuf {
        self.dir.join(&self.manifest.server_dir)
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
        progress: &impl BeginProgress
    ) -> Result<Self> {
        // create directory to contain instance
        if !instance_dir.exists() {
            fs::create_dir(instance_dir)?;
        }

        let instance = Self::new(
            instance_dir,
            ServerInstanceManifest {
                mc_version: mc_version.to_string(),
                server_dir: "server".to_string(),
                java_path: None,
                java_args: None,
                java_env: None,
                mod_loader
            }
        )?;

        // write instance manifest.json file
        instance.write_manifest()?;

        // create server directory before running installer
        let server_dir = instance.server_dir();
        fs::create_dir_all(&server_dir)?;

        let assets = AssetManager::new()?;

        if let Some(loader) = instance.manifest.mod_loader.as_ref() {
            let installer_jar = assets.download_installer_jar(&loader, progress)
                .await?;

            let mut cmd = LaunchCommand::new(&server_dir, None, None, None);
            cmd.arg("-jar").arg(installer_jar.to_string_lossy());

            match loader.name {
                ModLoaderName::Forge => cmd.arg("--installServer"),
                ModLoaderName::NeoForge => cmd.arg("--install-server")
            };

            cmd.spawn()?.wait()?;
        } else {
            let server_jar = server_dir.join("server.jar");
            assets.download_server_jar(mc_version, &server_jar, progress)
                .await?;
        }

        Ok(instance)
    }

    pub fn load(instance_dir: &Path) -> Result<Self> {
        let manifest_path = instance_dir.join(MANIFEST_FILE);
        if !manifest_path.exists() {
            bail!(Error::InstanceNotFound(instance_dir.to_str().unwrap().to_string()))
        }

        let json = fs::read_to_string(manifest_path)?;
        let manifest = serde_json::from_str::<ServerInstanceManifest>(json.as_str())?;

        Self::new(instance_dir, manifest)
    }

    pub async fn launch(&self) -> Result<Child> {
        let eula_path = self.server_dir().join("eula.txt");
        if !eula_path.exists() {
            // This seems wrong, I should be prompting the user or not doing this at all
            fs::write(eula_path, "eula=true")?;
        }

        let mut cmd = LaunchCommand::new(
            &self.server_dir(),
            self.manifest.java_path.as_ref(),
            self.manifest.java_args.as_ref(),
            self.manifest.java_env.as_ref()
        );

        if self.server_dir().join("user_jvm_args.txt").exists() {
            cmd.arg("@user_jvm_args.txt");
        }

        if let Some(loader) = &self.manifest.mod_loader {
            match loader.name {
                ModLoaderName::Forge => {
                    cmd.arg(format!("@libraries/net/minecraftforge/forge/{ver}/unix_args.txt", ver = loader.version));
                },
                ModLoaderName::NeoForge => {
                    cmd.arg(format!("@libraries/net/neoforged/neoforge/{ver}/unix_args.txt", ver = loader.version));
                }
            }
        } else {
            cmd.args(["-jar", "server.jar"]);
        }

        cmd.arg("nogui");

        Ok(cmd.spawn()?)
    }
}
