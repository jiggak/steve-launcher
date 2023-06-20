mod cli;
mod json;

use cli::{Parser, Cli, Commands};
use json::{VersionManifest, VersionManifestEntry, GameManifest};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), &'static str> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { dir, mc_version } =>
            create(&dir, &mc_version).await
    }
}

async fn get_version_manifest() -> Result<VersionManifest, reqwest::Error> {
    reqwest::get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .await?
        .json::<VersionManifest>()
        .await
}

async fn get_game_manifest(version: &VersionManifestEntry) -> Result<GameManifest, reqwest::Error> {
    reqwest::get(version.url.as_str())
        .await?
        .json::<GameManifest>()
        .await
}

async fn create(dir: &str, mc_version: &str) -> Result<(), &'static str> {
    let manifest = get_version_manifest()
        .await.map_err(|e| "get_version_manifest failed")?;

    let version = match manifest.versions.iter().find(|v| v.id == mc_version) {
        Some(version) => version,
        None => return Err("Version not found")
    };

    println!("downloading from {}", version.url);
    let game_manifest = get_game_manifest(version)
        .await.map_err(|e| "get_game_manifest failed")?;

    Ok(())
}