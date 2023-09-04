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

mod auth;
mod create;
mod launch;
mod modpack;

pub use {
    auth::msal_login,
    create::create_instance,
    launch::launch_instance,
    modpack::modpack_search_and_install,
    modpack::modpack_zip_install
};

use dialoguer::{Confirm, theme::ColorfulTheme};

fn console_theme() -> ColorfulTheme {
    ColorfulTheme::default()
}

fn prompt_confirm<S: Into<String>>(prompt: S) -> std::io::Result<bool> {
    Confirm::with_theme(&console_theme())
        .with_prompt(prompt)
        .wait_for_newline(true)
        .default(false)
        .interact()
}
