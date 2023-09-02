use std::{error::Error, path::Path};

use crate::ProgressHandler;
use steve::Instance;

pub async fn launch_instance(instance_dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut progress = ProgressHandler::new();

    let instance = Instance::load(&instance_dir)?;
    instance.launch(&mut progress)
        .await?;

    Ok(())
}
