/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2023 Josh Kropf <josh@slashdev.ca>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use anyhow::Result;

use steve::Account;

pub async fn msal_login() -> Result<()> {
    Account::login(|url, code| {
        println!("Open the URL in your browser and enter the code: {code}\n\t{url}");
    }).await?;

    Ok(())
}

pub fn clear_credentials() -> Result<()> {
    Account::clear()
}

pub fn print_account_status() -> Result<()> {
    let account = Account::load()?;

    println!("Account credentials exist");
    println!("   Mojang token refresh at {}", account.mc_token_expires());
    println!("      MSA token refresh at {}", account.msa_token_expires());

    Ok(())
}
