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
use std::{
    collections::HashMap, fs, path::{Path, PathBuf}, process::{Child, Command}
};

use crate::{
    account::Account, asset_client::AssetClient, asset_manager::{
        self, AssetManager, get_client_jar_path, make_forge_modded_jar
    },
    CurseForgeZip, env, Error, json::{
        CurseForgeFile, CurseForgeMod, ForgeDistribution, InstanceManifest,
        ModLoader, ModpackVersionManifest
    },
    Progress
};

const MANIFEST_FILE: &str = "manifest.json";

pub struct Instance {
    pub manifest: InstanceManifest,

    /// Absolute path of the instance directory
    pub dir: PathBuf
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
            manifest
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
        mod_loader: Option<ModLoader>
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
                mod_loader,
                custom_jar: None
            }
        )?;

        // write instance manifest.json file
        instance.write_manifest()?;

        Ok(instance)
    }

    pub async fn install_pack_zip(&self,
        pack: &CurseForgeZip,
        progress: &mut dyn Progress
    ) -> Result<(Vec<PathBuf>, Option<Vec<FileDownload>>)> {
        // copy pack overrides to minecraft dir
        pack.copy_game_data(&self.game_dir())?;

        let client = AssetClient::new();
        let file_ids = pack.manifest.get_file_ids();
        let project_ids = pack.manifest.get_project_ids();

        self.download_curseforge_files(&client, file_ids, project_ids, progress).await
    }

    pub async fn install_pack(&self,
        pack: &ModpackVersionManifest,
        progress: &mut dyn Progress
    ) -> Result<(Vec<PathBuf>, Option<Vec<FileDownload>>)> {
        let client = AssetClient::new();

        let assets: Vec<_> = pack.files.iter()
            .filter(|f| f.url.is_some())
            .collect();

        progress.begin("Downloading assets...", assets.len());

        for (i, f) in assets.iter().enumerate() {
            progress.advance(i + 1);

            // curse packs from modpacks.ch could include a single asset file
            // which is the full curse zip file, download and extract overrides
            if f.file_type == "cf-extract" {
                let dest_file_path = std::env::temp_dir().join(&f.name);
                client.download_file(f.url.as_ref().unwrap(), &dest_file_path).await?;

                let pack = CurseForgeZip::load_zip(&dest_file_path)?;
                pack.copy_game_data(&self.game_dir())?;

                continue;
            }

            let dest_file_path = self.game_dir()
                .join(&f.path)
                .join(&f.name);

            // save time/bandwidth and skip download if dest file exists
            if dest_file_path.exists() {
                continue;
            }

            client.download_file(f.url.as_ref().unwrap(), &dest_file_path).await?;
        }

        progress.end();

        let mods: Vec<_> = pack.files.iter()
            .filter_map(|f| f.curseforge.as_ref())
            .collect();

        let file_ids = mods.iter().map(|c| c.file_id).collect();
        let project_ids = mods.iter().map(|c| c.project_id).collect();

        self.download_curseforge_files(&client, file_ids, project_ids, progress).await
    }

    async fn download_curseforge_files(&self,
        client: &AssetClient,
        file_ids: Vec<u64>,
        project_ids: Vec<u64>,
        progress: &mut dyn Progress
    ) -> Result<(Vec<PathBuf>, Option<Vec<FileDownload>>)> {
        let mut file_list = client.get_curseforge_file_list(&file_ids).await?;
        let mut mod_list = client.get_curseforge_mods(&project_ids).await?;

        if file_list.len() != mod_list.len() {
            bail!(Error::CurseFileListMismatch {
                file_list_len: file_list.len(),
                mod_list_len: mod_list.len()
            });
        }

        // sort the lists so that we can zip them into list of pairs
        file_list.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
        mod_list.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));

        let file_downloads: Vec<_> = file_list.iter()
            .zip(mod_list)
            .map(|(f, m)| FileDownload::new(f, &m))
            .collect();

        // filter files that can be auto-downloaded, and those that must be manually downloaded
        let (downloads, blocked): (Vec<_>, Vec<_>) = file_downloads.clone().into_iter()
            .partition(|f| f.can_auto_download);

        progress.begin("Downloading mods...", downloads.len());

        // create mods dir in case there are zero automated downloads with one or more manual downloads
        fs::create_dir_all(self.mods_dir())?;

        for (i, f) in downloads.iter().enumerate() {
            progress.advance(i + 1);

            let dest_file_path = self.get_file_path(f);

            // save time/bandwidth and skip download if dest file exists
            if dest_file_path.exists() {
                continue;
            }

            client.download_file(&f.url, &dest_file_path).await?;
        }

        progress.end();

        let delete_files = [
            list_extra_files(&self.mods_dir(), &file_downloads)?,
            list_extra_files(&self.resource_pack_dir(), &file_downloads)?,
            list_extra_files(&self.shader_pack_dir(), &file_downloads)?
        ].concat();

        if !blocked.is_empty() {
            Ok((delete_files, Some(blocked)))
        } else {
            Ok((delete_files, None))
        }
    }

    pub fn load(instance_dir: &Path) -> Result<Instance> {
        let manifest_path = instance_dir.join(MANIFEST_FILE);
        if !manifest_path.exists() {
            bail!(Error::InstanceNotFound(instance_dir.to_str().unwrap().to_string()))
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

    pub fn get_file_type_dir(&self, file_type: &FileType) -> PathBuf {
        match file_type {
            FileType::Mod => self.mods_dir(),
            FileType::Resource => self.resource_pack_dir(),
            FileType::Shaders => self.shader_pack_dir()
        }
    }

    pub fn get_file_path(&self, file: &FileDownload) -> PathBuf {
        self.get_file_type_dir(&file.file_type).join(&file.file_name)
    }

    pub fn install_file(&self, file: &FileDownload, src_path: &Path) -> std::io::Result<()> {
        let dest_file = self.get_file_path(file);
        fs::copy(src_path, dest_file)?;
        Ok(())
    }

    pub async fn launch(&self, progress: &mut dyn Progress) -> Result<Child> {
        let account = Account::load_with_tokens().await?;

        let profile = account.fetch_profile().await?;

        let assets = AssetManager::new()?;

        let game_manifest = assets.get_game_manifest(&self.manifest.mc_version).await?;
        let asset_manifest = assets.get_asset_manfiest(&game_manifest).await?;

        let loader_manifest = match &self.manifest.mod_loader {
            Some(mod_loader) => Some(assets.get_loader_manifest(mod_loader).await?),
            None => None
        };

        assets.download_assets(&asset_manifest, progress).await?;
        assets.download_libraries(&game_manifest, progress).await?;

        if let Some(loader_manifest) = &loader_manifest {
            assets.download_loader_libraries(loader_manifest, progress).await?;
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

        let mut cmd = LaunchCommand::new(self);
        fs::create_dir_all(self.game_dir())?;

        let mut main_jar: String = get_client_jar_path(&game_manifest.id);

        if let Some(loader_manifest) = &loader_manifest {
            match &loader_manifest.dist {
                // legacy forge distributions required modifying the `minecraft.jar` file
                ForgeDistribution::Legacy { jar_mods, fml_libs } => {
                    main_jar = make_forge_modded_jar(&main_jar, &loader_manifest.version, &jar_mods)
                        ?.to_string_lossy().to_string();

                    // forge will throw an error on startup attempting to download
                    // these libraries (404 not found), unless they already exist
                    if let Some(fml_libs) = fml_libs {
                        super::fs::copy_files(
                            fml_libs.iter()
                                .map(|l| env::get_libs_dir().join(l.asset_path())),
                            self.fml_libs_dir()
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
                },
                ForgeDistribution::Current { main_class, minecraft_arguments, .. } => {
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
            game_manifest.libraries.iter()
                .filter(|lib| lib.has_rules_match())
                .filter_map(|lib| lib.downloads.artifact.as_ref())
                .map(|a| a.path.clone())
        );

        if let Some(loader_manifest) = &loader_manifest {
            if let ForgeDistribution::Current { libraries, .. } = &loader_manifest.dist {
                libs.extend(
                    libraries.iter()
                        .map(|lib| lib.asset_path())
                );
            }
        }

        let classpath = std::env::join_paths(
            asset_manager::dedup_libs(&libs)?.iter()
                .map(|p| env::get_libs_dir().join(p))
        )?;

        let auth_session = format!("token:{token}:{profileId}",
            token = account.access_token(), profileId = profile.id);

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

struct LaunchCommand {
    cmd: Command,
    ctx: HashMap<&'static str, String>,
    args: Vec<String>
}

impl LaunchCommand {
    fn new(instance: &Instance) -> Self {
        // use java override path from instance manifest, or default to "java" in PATH
        let mut cmd = if let Some(path) = &instance.manifest.java_path {
            Command::new(path)
        } else {
            Command::new("java")
        };

        if let Some(args) = &instance.manifest.java_args {
            cmd.args(args);
        }

        // set current directory for log output
        cmd.current_dir(instance.game_dir());

        Self {
            cmd: cmd,
            ctx: HashMap::new(),
            args: Vec::new()
        }
    }

    fn arg_ctx<S: Into<String>>(&mut self, key: &'static str, val: S) -> &mut Self {
        self.ctx.insert(key, val.into());
        self
    }

    fn arg<S: Into<String>>(&mut self, val: S) -> &mut Self {
        self.args.push(val.into());
        self
    }

    fn args<I>(&mut self, iter: I) -> &mut Self
        where I: IntoIterator, I::Item: Into<String>
    {
        iter.into_iter().for_each(|v| self.args.push(v.into()));
        self
    }

    fn spawn(&mut self) -> std::io::Result<Child> {
        for arg in &self.args {
            self.cmd.arg(
                shellexpand::env_with_context_no_errors(
                    &arg,
                    |var:&str| self.ctx.get(var)
                ).to_string()
            );
        }

        self.cmd.spawn()
    }
}

#[derive(Clone)]
pub enum FileType {
    Mod,
    Resource,
    Shaders
}

#[derive(Clone)]
pub struct FileDownload {
    pub file_name: String,
    pub file_type: FileType,
    pub can_auto_download: bool,
    pub url: String
}

impl FileDownload {
    pub fn new(f: &CurseForgeFile, m: &CurseForgeMod) -> Self {
        // it feels brittle using hard coded classId, but I don't see anything
        // else that can differentiate mods|resource pack|etc
        let file_type = match m.class_id {
            6 => FileType::Mod,
            12 => FileType::Resource,
            6552 => FileType::Shaders,
            x => panic!("Unimplemented curseforge class_id {x}")
        };

        // url for user to download the file manually
        let user_dl_url = format!("{site_url}/download/{file_id}",
            site_url = m.links.website_url, file_id = f.file_id);

        FileDownload {
            file_name: f.file_name.clone(),
            file_type,
            can_auto_download: f.download_url.is_some(),
            url: match &f.download_url {
                Some(v) => v.clone(),
                None => user_dl_url
            }
        }
    }
}

fn list_extra_files(dir: &Path, downloads: &Vec<FileDownload>) -> Result<Vec<PathBuf>> {
    let mut delete_files: Vec<PathBuf> = vec![];

    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;

            if !downloads.iter().any(|f| f.file_name == entry.file_name().to_string_lossy()) {
                delete_files.push(entry.path());
            }
        }
    }

    Ok(delete_files)
}
