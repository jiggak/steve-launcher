use std::error::Error as StdError;

use super::account::{Account, LoginCallback};

pub async fn login(callback: LoginCallback) -> Result<(), Box<dyn StdError>> {
    Account::login(callback).await?;

    Ok(())
}
