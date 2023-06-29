use std::{fs, path::Path, path::PathBuf};

use std::error::Error as StdError;

use crate::json::InstanceManifest;

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

    pub fn create(instance_dir: &Path, mc_version: &str) -> Result<Instance, Box<dyn StdError>> {
        // create directory to contain instance
        fs::create_dir(instance_dir)?;

        let instance = Instance::new(
            instance_dir,
            InstanceManifest {
                mc_version: mc_version.to_string(),
                game_dir: "minecraft".to_string(),
                java_path: None
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
}
