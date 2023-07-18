use serde::Deserialize;

#[derive(Deserialize)]
pub struct ForgeVersionManifest {
    pub versions: Vec<ForgeVersionManifestEntry>
}

#[derive(Deserialize, Clone)]
pub struct ForgeVersionManifestEntry {
    pub recommended: bool,
    #[serde(rename(deserialize = "releaseTime"))]
    pub release_time: String,
    pub requires: Vec<ForgeVersionRequires>,
    pub sha256: String,
    pub version: String
}

impl ForgeVersionManifestEntry {
    pub fn is_for_mc_version(&self, mc_version: &str) -> bool {
        self.requires.iter().any(|r| r.equals == mc_version)
    }
}

#[derive(Deserialize, Clone)]
pub struct ForgeVersionRequires {
    pub equals: String,
    pub uid: String
}
