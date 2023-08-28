use std::{
    collections::HashMap, error::Error as StdError, fs, path::{Path, PathBuf},
    process::Child, process::Command
};

use crate::{
    account::Account, asset_client::AssetClient, asset_manager::{self, AssetManager},
    env, Error,
    json::{
        CurseForgeFile, CurseForgeMod, InstanceManifest, ModpackVersionManifest
    },
    Progress, CurseForgeZip
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

    pub fn exists(instance_dir: &Path) -> bool {
        if !instance_dir.exists() || !instance_dir.is_dir() {
            return false;
        }

        instance_dir.join(MANIFEST_FILE).exists()
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

    pub async fn install_pack_zip(&self,
        pack: &CurseForgeZip,
        progress: &mut dyn Progress
    ) -> Result<Option<Vec<FileDownload>>, Box<dyn StdError>> {
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
    ) -> Result<Option<Vec<FileDownload>>, Box<dyn StdError>> {
        let client = AssetClient::new();

        let assets: Vec<_> = pack.files.iter()
            .filter(|f| f.url.is_some())
            .collect();

        progress.begin("Downloading assets...", assets.len());

        for (i, f) in assets.iter().enumerate() {
            progress.advance(i + 1);

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
    ) -> Result<Option<Vec<FileDownload>>, Box<dyn StdError>> {
        let mut file_list = client.get_curseforge_file_list(&file_ids).await?;
        let mut mod_list = client.get_curseforge_mods(&project_ids).await?;

        if file_list.len() != mod_list.len() {
            let msg = format!("CurseForge file results({}) do not match mod results({})",
                file_list.len(), mod_list.len());
            return Err(Box::new(Error::new(msg)));
        }

        // sort the lists so that we can zip them into list of pairs
        file_list.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
        mod_list.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));

        // filter files that can be auto-downloaded, and those that must be manually downloaded
        let (downloads, blocked): (Vec<_>, Vec<_>) = file_list.iter().zip(mod_list)
            .map(|(f, m)| FileDownload::new(f, &m).unwrap())
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

        if !blocked.is_empty() {
            Ok(Some(blocked))
        } else {
            Ok(None)
        }
    }

    pub fn load(instance_dir: &Path) -> Result<Instance, Box<dyn StdError>> {
        let manifest_path = instance_dir.join(MANIFEST_FILE);
        let json = fs::read_to_string(manifest_path)?;
        let manifest = serde_json::from_str::<InstanceManifest>(json.as_str())?;

        Instance::new(instance_dir, manifest)
    }

    pub fn set_versions(&mut self, mc_version: String, forge_version: Option<String>) -> Result<(), Box<dyn StdError>> {
        self.manifest.mc_version = mc_version;
        self.manifest.forge_version = forge_version;
        self.write_manifest()
    }

    pub fn game_dir(&self) -> PathBuf {
        self.dir.join(&self.manifest.game_dir)
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

    pub fn natives_dir(&self) -> PathBuf {
        self.dir.join("natives")
    }

    pub fn get_file_type_dir(&self, file_type: &FileType) -> PathBuf {
        match file_type {
            FileType::Mod => self.mods_dir(),
            FileType::Resource => self.resource_pack_dir()
        }
    }

    pub fn get_file_path(&self, file: &FileDownload) -> PathBuf {
        self.get_file_type_dir(&file.file_type).join(&file.file_name)
    }

    pub fn install_file(&self, file: &FileDownload, src_path: &Path) -> std::io::Result<()> {
        let dest_file = self.get_file_path(&file);
        fs::copy(src_path, dest_file)?;
        Ok(())
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

pub enum FileType {
    Resource,
    Mod
}

pub struct FileDownload {
    pub file_name: String,
    pub file_type: FileType,
    pub can_auto_download: bool,
    pub url: String
}

impl FileDownload {
    pub fn new(f: &CurseForgeFile, m: &CurseForgeMod) -> Result<Self, Error> {
        // it feels brittle using hard coded classId, but I don't see anything
        // else that can differentiate mods|resource pack|etc
        let file_type = match m.class_id {
            6 => Ok(FileType::Mod),
            12 => Ok(FileType::Resource),
            x => Err(Error::new(format!("Unknown class_id {}", x)))
        }?;

        // url for user to download the file manually
        let user_dl_url = format!("{site_url}/download/{file_id}",
            site_url = m.links.website_url, file_id = f.file_id);

        Ok(FileDownload {
            file_name: f.file_name.clone(),
            file_type,
            can_auto_download: f.download_url.is_some(),
            url: match &f.download_url {
                Some(v) => v.clone(),
                None => user_dl_url
            }
        })
    }
}
