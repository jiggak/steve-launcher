use serde::Deserialize;

#[derive(Deserialize)]
pub struct CurseForgePack {
    pub minecraft: CurseForgeMinecraft,
    #[serde(rename(deserialize = "manifestType"))]
    pub manifest_type: String,
    #[serde(rename(deserialize = "manifestVersion"))]
    pub manifest_version: u8,
    pub name: String,
    pub version: String,
    pub author: String,
    pub files: Vec<CurseForgePackFile>,
    pub overrides: String
}

impl CurseForgePack {
    pub fn get_file_ids(&self) -> Vec<u64> {
        self.files.iter()
            .map(|f| f.file_id)
            .collect()
    }
}

#[derive(Deserialize)]
pub struct CurseForgeMinecraft {
    pub version: String,
    #[serde(rename(deserialize = "modLoaders"))]
    pub mod_loaders: Vec<CurseForgeModloader>
}

impl CurseForgeMinecraft {
    pub fn get_forge_version(&self) -> Option<String> {
        let loader = self.mod_loaders.iter().find(|l| l.id.starts_with("forge"));
        if let Some(loader) = loader {
            Some(loader.id.replace("forge-", ""))
        } else {
            None
        }
    }
}

#[derive(Deserialize)]
pub struct CurseForgeModloader {
    pub id: String,
    pub primary: bool
}

#[derive(Deserialize)]
pub struct CurseForgePackFile {
    #[serde(rename(deserialize = "projectID"))]
    pub project_id: u64,
    #[serde(rename(deserialize = "fileID"))]
    pub file_id: u64,
    pub required: bool
}

#[derive(Deserialize)]
pub struct CurseForgeResponse<T> {
    pub data: Vec<T>
}

#[derive(Deserialize)]
// https://docs.curseforge.com/#tocS_File
pub struct CurseForgeFile {
    #[serde(rename(deserialize = "id"))]
    pub file_id: u64,
    #[serde(rename(deserialize = "modId"))]
    pub mod_id: u64,
    #[serde(rename(deserialize = "fileName"))]
    pub file_name: String,
    #[serde(rename(deserialize = "downloadUrl"))]
    pub download_url: Option<String>
}

#[derive(Deserialize)]
// https://docs.curseforge.com/#tocS_Mod
pub struct CurseForgeMod {
    #[serde(rename(deserialize = "id"))]
    pub mod_id: u64,
    pub slug: String,
    pub links: CurseForgeModLinks
}

#[derive(Deserialize)]
pub struct CurseForgeModLinks {
    #[serde(rename(deserialize = "websiteUrl"))]
    pub website_url: String,
    #[serde(rename(deserialize = "wikiUrl"))]
    pub wiki_url: Option<String>,
    #[serde(rename(deserialize = "issuesUrl"))]
    pub issues_url: Option<String>,
    #[serde(rename(deserialize = "sourceUrl"))]
    pub source_url: Option<String>
}
