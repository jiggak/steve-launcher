use std::{fs, path::{Path, PathBuf}};

use anyhow::Result;

use crate::{curseforge_hash::curseforge_hash, CurseClient, Error};

pub struct ModsManager {
    mods_dir: PathBuf,
    pub mods: Vec<Mod>
}

pub struct Mod {
    pub file_name: String,
    pub mod_id: u32
}

impl ModsManager {
    pub async fn load_curseforge_mods<P: Into<PathBuf>>(mods_dir: P) -> Result<Self> {
        let mods_dir = mods_dir.into();

        // generate hashes for all files in mods_dir
        let hashes = list_file_hashes(&mods_dir)?;
        let only_hashes = hashes.iter()
            .map(|(_, h)| *h)
            .collect();

        let client = CurseClient::new();

        let results = client.get_fingerprints(&only_hashes).await?;

        // closure to get/map fingerprint results to mod manager state
        let get_result = |file_name: &str, hash: u32| {
            results.exact_matches.iter()
                .find(|r| r.file.file_fingerprint == hash)
                .map(|r| Mod {
                    file_name: file_name.to_string(),
                    mod_id: r.file.mod_id
                })
        };

        let mods: Result<Vec<_>, _> = hashes.into_iter()
            .map(|(f, h)|
                get_result(&f, h)
                    .ok_or_else(|| Error::MissingFingerprint(f, h))
            )
            .collect();

        Ok(Self { mods_dir, mods: mods? })
    }

    pub fn install_mod(&self, mod_id: u32, file_id: u32) -> Result<()> {
        let existing = self.mods.iter().find(|m| m.mod_id == mod_id);
        if let Some(existing) = existing {
            fs::remove_file(self.mods_dir.join(&existing.file_name))?;
        }

        Ok(())
    }
}

fn list_file_hashes(mods_dir: &Path) -> Result<Vec<(String, u32)>> {
    let mut hashes: Vec<_> = Vec::new();

    for entry in fs::read_dir(mods_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap()
            .to_string_lossy()
            .to_string();

        let data = fs::read(&path)?;
        let hash = curseforge_hash(&data);

        hashes.push((file_name, hash));
    }

    Ok(hashes)
}
