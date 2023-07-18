use std::{fs, path::Path, path::PathBuf};
use std::error::Error as StdError;

use crate::{env, Error, commands::Progress, downloader::Downloader};
use crate::json::{AssetDownload, AssetManifest, GameManifest, ForgeManifest};

pub struct AssetManager {
    downloader: Downloader,
    assets_dir: PathBuf,
    cache_dir: PathBuf,
    libs_dir: PathBuf
}

impl AssetManager {
    pub fn new() -> Result<Self, Box<dyn StdError>> {
        let manager = AssetManager {
            downloader: Downloader::new(),
            assets_dir: env::get_assets_dir(),
            cache_dir: env::get_cache_dir(),
            libs_dir: env::get_libs_dir()
        };

        fs::create_dir_all(manager.objects_dir())?;
        fs::create_dir_all(manager.indexes_dir())?;
        fs::create_dir_all(manager.versions_dir())?;

        Ok(manager)
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.assets_dir.join("objects")
    }

    pub fn indexes_dir(&self) -> PathBuf {
        self.assets_dir.join("indexes")
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.cache_dir.join("versions")
    }

    pub fn virtual_assets_dir(&self, asset_index_id: &str) -> PathBuf {
        self.assets_dir.join("virtual").join(asset_index_id)
    }

    pub fn get_client_jar_path(mc_version: &str) -> String {
        format!("com/mojang/minecraft/{mc_version}/minecraft-{mc_version}-client.jar")
    }

    pub async fn get_game_manifest(&self, mc_version: &str) -> Result<GameManifest, Box<dyn StdError>> {
        let version_file_path = self.versions_dir()
            .join(format!("{mc_version}.json"));

        if !version_file_path.exists() {
            let game_manifest_json = self.downloader.get_game_manifest_json(mc_version).await?;

            fs::write(&version_file_path, game_manifest_json)?;
        }

        let version_file = fs::File::open(version_file_path)?;
        let game_manifest: GameManifest = serde_json::from_reader(version_file)?;

        Ok(game_manifest)
    }

    pub async fn get_forge_manifest(&self, forge_version: &str) -> Result<ForgeManifest, Box<dyn StdError>> {
        let version_file_path = self.versions_dir()
            .join(format!("forge_{forge_version}.json"));

        if !version_file_path.exists() {
            let json = self.downloader.get_forge_manifest_json(forge_version).await?;

            fs::write(&version_file_path, json)?;
        }

        let version_file = fs::File::open(version_file_path)?;
        let forge_manifest: ForgeManifest = serde_json::from_reader(version_file)?;

        Ok(forge_manifest)
    }

    pub async fn get_asset_manfiest(&self, game_manifest: &GameManifest) -> Result<AssetManifest, Box<dyn StdError>> {
        let index_file_path = self.indexes_dir()
            .join(format!("{ver}.json", ver = game_manifest.asset_index.id));

        if index_file_path.exists() {
            let index_file = fs::File::open(&index_file_path)?;
            return Ok(serde_json::from_reader(index_file)?);
        }

        let asset_index_url = game_manifest.asset_index.download.url.as_str();
        let asset_manifest = self.downloader.get_asset_manfiest(asset_index_url).await?;

        let index_file = fs::File::create(index_file_path)?;
        serde_json::to_writer(index_file, &asset_manifest)?;

        Ok(asset_manifest)
    }

    pub async fn download_assets(&self,
        asset_manifest: &AssetManifest,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        progress.begin("Downloading assets", asset_manifest.objects.len());

        for (i, (_, obj)) in asset_manifest.objects.iter().enumerate() {
            progress.advance(i + 1);
            self.download_asset(&obj.hash).await?;
        }

        progress.end();

        Ok(())
    }

    async fn download_asset(&self, hash: &str) -> Result<(), Box<dyn StdError>> {
        // first 2 chars of hash is used for directory of objects
        let hash_prefix = &hash[0..2];

        let object_file = self.objects_dir()
            .join(hash_prefix)
            .join(hash);

        // skip download if object file already exists
        if object_file.exists() {
            return Ok(());
        }

        fs::create_dir_all(object_file.parent().unwrap())?;

        let url = format!("https://resources.download.minecraft.net/{hash_prefix}/{hash}");

        self.downloader.download_file(&url, &object_file).await
    }

    pub async fn download_libraries(&self,
        game_manifest: &GameManifest,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        let client = game_manifest.downloads.get("client")
            .ok_or(Error::new("Missing 'client' key in downloads object"))?;

        let client_path = Self::get_client_jar_path(&game_manifest.id);
        let mut lib_downloads: Vec<(&str, &AssetDownload)> = vec![
            (client_path.as_str(), client)
        ];

        lib_downloads.extend(
            game_manifest.libraries.iter()
                .filter(|lib| lib.has_rules_match())
                .flat_map(|lib| {
                    // FIXME I think this can be simplified
                    let mut result = vec![];

                    if let Some(artifact) = &lib.downloads.artifact {
                        result.push(artifact);
                    }

                    if let Some(natives) = lib.natives_artifact() {
                        result.push(natives);
                    }

                    if result.is_empty() {
                        panic!("unhandled download for {}", lib.name);
                    }

                    return result;
                })
                .map(|a| (a.path.as_str(), &a.download))
        );

        progress.begin("Downloading libraries", lib_downloads.len());

        for (i, (path, dl)) in lib_downloads.iter().enumerate() {
            progress.advance(i + 1);
            self.download_library(path, dl).await?;
        }

        progress.end();

        Ok(())
    }

    pub async fn download_forge_libraries(&self,
        forge_manifest: &ForgeManifest,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        let downloads: Vec<_> = forge_manifest.libraries.iter()
            .chain(forge_manifest.maven_files.iter())
            .map(|lib| &lib.downloads.artifact)
            .map(|a| (a.asset_path(), &a.download))
            .collect();

        progress.begin("Downloading forge libraries", downloads.len());

        for (i, (path, dl)) in downloads.iter().enumerate() {
            progress.advance(i + 1);
            self.download_library(path, dl).await?;
        }

        progress.end();

        Ok(())
    }

    async fn download_library(&self, path: &str, download: &AssetDownload) -> Result<(), Box<dyn StdError>> {
        let lib_file = self.libs_dir.join(path);

        // skip download if lib file already exists
        if lib_file.exists() {
            return Ok(());
        }

        fs::create_dir_all(lib_file.parent().unwrap())?;

        self.downloader.download_file(&download.url, &lib_file).await
    }

    pub fn copy_resources(&self,
        asset_manifest: &AssetManifest,
        target_dir: &Path,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        progress.begin("Copy resources", asset_manifest.objects.len());

        for (i, (path, obj)) in asset_manifest.objects.iter().enumerate() {
            let object_path = self.objects_dir()
                .join(&obj.hash[0..2])
                .join(&obj.hash);

            let resource_path = target_dir.join(path);

            if !resource_path.exists() {
                fs::create_dir_all(resource_path.parent().unwrap())?;
                fs::copy(object_path, resource_path)?;
            }

            progress.advance(i + 1);
        }

        progress.end();

        Ok(())
    }

    pub fn extract_natives(self,
        game_manifest: &GameManifest,
        target_dir: &Path,
        progress: &mut dyn Progress
    ) -> Result<(), Box<dyn StdError>> {
        let native_libs: Vec<_> = game_manifest.libraries.iter()
            .filter(|lib| lib.has_rules_match())
            .filter_map(|lib| lib.natives_artifact())
            .collect();

        progress.begin("Extracting native jars", native_libs.len());

        for (i, lib) in native_libs.iter().enumerate() {
            let lib_file = self.libs_dir.join(&lib.path);
            zip_extract::extract(fs::File::open(lib_file)?, target_dir, false)?;
            progress.advance(i + 1);
        }

        progress.end();

        Ok(())
    }
}
