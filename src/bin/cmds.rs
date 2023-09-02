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
