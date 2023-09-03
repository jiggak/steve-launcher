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

use std::{error::Error as StdError, fs::{self, File}, io, path::{Path, PathBuf}};
use crate::json::CurseForgePack;

pub struct CurseForgeZip {
    pub manifest: CurseForgePack,
    zip_temp_dir: PathBuf
}

impl CurseForgeZip {
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

impl CurseForgeZip {
    pub fn copy_game_data(&self, game_dir: &Path) -> io::Result<()> {
        copy_dir_all(self.zip_temp_dir.join(&self.manifest.overrides), game_dir)
    }
}

impl Drop for CurseForgeZip {
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
