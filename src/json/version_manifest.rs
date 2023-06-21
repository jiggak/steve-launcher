use serde::Deserialize;

#[derive(Deserialize)]
pub struct VersionManifest {
    pub versions: Vec<VersionManifestEntry>
}

#[derive(Deserialize)]
pub struct VersionManifestEntry {
    pub id: String,
    #[serde(rename(deserialize = "type"))]
    pub release_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename(deserialize = "releaseTime"))]
    pub release_time: String,
    pub sha1: String,
    #[serde(rename(deserialize = "complianceLevel"))]
    pub compliance_level: u8
}