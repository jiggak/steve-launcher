use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct GameManifest {
    pub arguments: Option<GameArgsIndex>,
    #[serde(rename(deserialize = "assetIndex"))]
    pub asset_index: GameAssetIndex,
    pub assets: String,
    #[serde(rename(deserialize = "complianceLevel"))]
    pub compliance_level: u8,
    pub downloads: HashMap<String, AssetDownload>,
    pub id: String,
    #[serde(rename(deserialize = "javaVersion"))]
    pub java_version: GameJavaVersion,
    pub libraries: Vec<GameLibrary>,
    pub logging: GameLogging,
    #[serde(rename(deserialize = "mainClass"))]
    pub main_class: String,
    #[serde(rename(deserialize = "minecraftArguments"))]
    pub minecraft_arguments: Option<String>,
    #[serde(rename(deserialize = "minimumLauncherVersion"))]
    pub minimum_launcher_version: u8,
    #[serde(rename(deserialize = "releaseTime"))]
    pub release_time: String,
    pub time: String,
    #[serde(rename(deserialize = "type"))]
    pub release_type: String
}

#[derive(Deserialize)]
pub struct GameArgsIndex {
    pub game: GameArgs,
    pub jvm: GameArgs
}

#[derive(Deserialize)]
#[serde(from = "GameArgsRaw")]
pub struct GameArgs(pub Vec<GameArg>);

#[derive(Deserialize)]
struct GameArgsRaw(Vec<GameArgTypes>);

#[derive(Deserialize)]
#[serde(untagged)]
enum GameArgTypes {
    String(String),
    GameArg(GameArg)
}

#[derive(Deserialize, Clone)]
pub struct GameArg {
    pub value: GameArgValue,
    pub rules: Vec<GameArgRule>
}

impl GameArg {
    fn new<S: Into<String>>(v: S) -> Self {
        Self {
            value: GameArgValue::Single(v.into()),
            rules: Vec::new()
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum GameArgValue {
    Single(String),
    Many(Vec<String>)
}

#[derive(Deserialize, Clone)]
pub struct GameArgRule {
    pub action: String,
    pub features: Option<HashMap<String, bool>>,
    pub os: Option<OsProperties>
}

#[derive(Deserialize, Clone)]
pub struct OsProperties {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>
}

impl From<GameArgsRaw> for GameArgs {
    fn from(args: GameArgsRaw) -> Self {
        GameArgs(args.0.iter().map(|elem| {
            match elem {
                GameArgTypes::String(v) => GameArg::new(v),
                GameArgTypes::GameArg(v) => v.clone()
            }
        }).collect())
    }
}

#[derive(Deserialize)]
pub struct GameAssetIndex {
    pub id: String,
    #[serde(flatten)]
    pub download: AssetDownload,
    #[serde(rename(deserialize = "totalSize"))]
    pub total_size: u64
}

#[derive(Deserialize)]
pub struct AssetDownload {
    pub sha1: String,
    pub size: u32,
    pub url: String
}

#[derive(Deserialize)]
pub struct GameJavaVersion {
    pub component: String,
    #[serde(rename(deserialize = "majorVersion"))]
    pub major_version: u8
}

#[derive(Deserialize)]
pub struct GameLibrary {
    pub downloads: GameLibraryDownloads,
    pub extract: Option<GameLibraryExtract>,
    pub name: String,
    pub natives: Option<HashMap<String, String>>,
    pub rules: Option<Vec<GameLibraryRule>>,
}

#[derive(Deserialize)]
pub struct GameLibraryDownloads {
    pub artifact: Option<GameLibraryArtifact>,
    pub classifiers: Option<HashMap<String, GameLibraryArtifact>>
}

#[derive(Deserialize)]
pub struct GameLibraryArtifact {
    pub path: String,
    #[serde(flatten)]
    pub download: AssetDownload
}

#[derive(Deserialize)]
pub struct GameLibraryExtract {
    pub exclude: Vec<String>
}

#[derive(Deserialize)]
pub struct GameLibraryRule {
    pub action: String,
    pub os: Option<OsProperties>
}

#[derive(Deserialize)]
pub struct GameLogging {
    pub client: GameLoggingClient
}

#[derive(Deserialize)]
pub struct GameLoggingClient {
    pub argument: String,
    pub file: GameLoggingArtifact,
    #[serde(rename(deserialize = "type"))]
    pub file_type: String
}

#[derive(Deserialize)]
pub struct GameLoggingArtifact {
    pub id: String,
    #[serde(flatten)]
    pub download: AssetDownload
}
