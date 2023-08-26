use std::{error::Error as StdError, fs::{self, File}, io, path::{Path, PathBuf}};
use crate::json::CurseForgePack;

pub struct ModPack {
    pub manifest: CurseForgePack,
    zip_temp_dir: PathBuf
}

impl ModPack {
    pub fn load_zip(zip_path: &Path) -> Result<Self, Box<dyn StdError>> {
        let zip_temp_dir = zip_path.file_stem().unwrap();

        // extract zip to temp dir
        let zip_temp_dir = std::env::temp_dir().join(zip_temp_dir);
        zip_extract::extract(File::open(zip_path)?, &zip_temp_dir, false)?;

        // read modpack manifest
        let manifest: CurseForgePack = serde_json::from_reader(
            File::open(zip_temp_dir.join("manifest.json"))?
        )?;

        Ok(Self {
            manifest,
            zip_temp_dir
        })
    }
}

impl ModPack {
    pub fn copy_game_data(&self, game_dir: &Path) -> io::Result<()> {
        copy_dir_all(self.zip_temp_dir.join(&self.manifest.overrides), game_dir)
    }
}

impl Drop for ModPack {
    fn drop(&mut self) {
        // delete temp dir created
        fs::remove_dir_all(&self.zip_temp_dir).unwrap();
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
