use std::error::Error;

use steve::Account;

pub async fn msal_login() -> Result<(), Box<dyn Error>> {
    Account::login(|url, code| {
        println!("Open the URL in your browser and enter the code: {code}\n\t{url}");
    }).await.map(|_| ())
}
