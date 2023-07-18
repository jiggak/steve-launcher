use serde::Deserialize;

#[derive(Deserialize)]
pub struct ForgeVersionManifest {
    pub versions: Vec<ForgeVersionManifestEntry>
}

#[derive(Deserialize)]
pub struct ForgeVersionManifestEntry {
    pub recommended: bool,
    #[serde(rename(deserialize = "releaseTime"))]
    pub release_time: String,
    pub requires: Vec<ForgeVersionRequires>,
    pub sha256: String,
    pub version: String
}

#[derive(Deserialize)]
pub struct ForgeVersionRequires {
    pub equals: String,
    pub uid: String
}
