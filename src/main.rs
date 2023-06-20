mod mc_json;

use mc_json::{VersionManifest, GameManifest};
use serde_json;

#[tokio::main(flavor = "current_thread")]
async fn main2() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = reqwest::get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .await?
        .json::<VersionManifest>()
        .await?;

    println!("{}", manifest.versions[0].url);

    let game_manifest = reqwest::get(manifest.versions[0].url.as_str())
        .await?
        .json::<GameManifest>()
        .await?;

    for lib in game_manifest.libraries {
        println!("{:?}", lib.name);
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::open("1.7.10.json")?;
    let reader = std::io::BufReader::new(file);

    let game_manifest: GameManifest = serde_json::from_reader(reader)?;

    for lib in game_manifest.libraries.iter() {
        println!("{:?}", lib.name);
    }

    Ok(())
}
