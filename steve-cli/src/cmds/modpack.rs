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

use console::Term;
use dialoguer::Select;
use std::{
    error::Error, io::Result as IoResult, path::Path, process::{Command, Stdio},
    sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{self, Sender}},
    thread::{self, Scope}
};

use crate::ProgressHandler;
use steve::{
    AssetClient, CurseForgeZip, DownloadWatcher, FileDownload, Instance,
    ModpackManifest, ModpackVersion, Progress, WatcherMessage
};
use super::{console_theme, prompt_confirm};

pub async fn modpack_search_and_install(
    instance_dir: &Path,
    search: &str,
    limit: u8
) -> Result<(), Box<dyn Error>> {
    let mut progress = ProgressHandler::new();
    let client = AssetClient::new();

    let results = client.search_modpacks(search, limit).await?;

    let mut search_results = Vec::new();

    progress.begin("Retrieving search results", results.total as usize);
    let mut count:usize = 0;

    for pack_id in results.pack_ids {
        search_results.push(
            client.get_ftb_modpack_versions(pack_id).await?
        );

        count += 1;
        progress.advance(count);
    }

    for curse_id in results.curseforge_ids {
        search_results.push(
            client.get_curse_modpack_versions(curse_id).await?
        );

        count += 1;
        progress.advance(count);
    }

    progress.end();

    let selection = Select::with_theme(&console_theme())
        .items(&format_modpack_results(search_results.iter()))
        .default(0)
        .interact()?;

    let selected_pack = &search_results[selection];

    let selection = Select::with_theme(&console_theme())
        .with_prompt("Select modpack version")
        .items(&format_modpack_versions(selected_pack.versions.iter()))
        .default(0)
        .interact()?;

    let selected_version = &selected_pack.versions[selection];

    let pack = if selected_pack.release_type == "Curseforge" {
        client.get_curse_modpack(selected_pack.pack_id, selected_version.version_id).await?
    } else {
        client.get_ftb_modpack(selected_pack.pack_id, selected_version.version_id).await?
    };

    let instance = if Instance::exists(&instance_dir) {
        if !prompt_confirm("Instance already exists, are you sure you want to install the pack here?")? {
            return Ok(())
        }

        let mut instance = Instance::load(&instance_dir)?;

        instance.set_versions(
            pack.get_minecraft_version()?,
            pack.get_forge_version()
        )?;

        instance
    } else {
        Instance::create(
            &instance_dir,
            &pack.get_minecraft_version()?,
            pack.get_forge_version()
        ).await?
    };

    let downloads = instance.install_pack(&pack, &mut progress)
        .await?;

    if let Some(downloads) = downloads {
        download_blocked(instance, downloads)?;
    }

    Ok(())
}

pub async fn modpack_zip_install(
    instance_dir: &Path,
    zip_file: &Path
) -> Result<(), Box<dyn Error>> {
    let mut progress = ProgressHandler::new();

    let pack = CurseForgeZip::load_zip(&zip_file)?;

    let instance = if Instance::exists(&instance_dir) {
        if !prompt_confirm("Instance already exists, are you sure you want to install the pack here?")? {
            return Ok(())
        }

        let mut instance = Instance::load(&instance_dir)?;

        instance.set_versions(
            pack.manifest.minecraft.version.clone(),
            pack.manifest.minecraft.get_forge_version()
        )?;

        instance
    } else {
        Instance::create(
            &instance_dir,
            &pack.manifest.minecraft.version,
            pack.manifest.minecraft.get_forge_version()
        ).await?
    };

    let downloads = instance.install_pack_zip(&pack, &mut progress)
        .await?;

    if let Some(downloads) = downloads {
        download_blocked(instance, downloads)?;
    }

    Ok(())
}

fn download_blocked(instance: Instance, downloads: Vec<FileDownload>) -> Result<(), Box<dyn Error>> {
    let watcher = DownloadWatcher::new(
        downloads.iter()
            .map(|f| f.file_name.as_str())
    );

    // copy any downloads already in watch dir
    for f in &downloads {
        if watcher.is_file_complete(&f.file_name) {
            let file_path = watcher.watch_dir.join(&f.file_name);
            instance.install_file(f, &file_path)?;
        }
    }

    if watcher.is_all_complete() {
        return Ok(());
    }

    let term = Term::stdout();
    term.hide_cursor()?;

    term.write_line("Files below must be downloaded manually. Press [o] to open all, [x] to quit.")?;

    print_download_state(&term, &watcher, &downloads)?;

    let (tx, rx) = mpsc::channel();

    thread::scope(|scope| -> IoResult<()> {
        let watch_cancel = watcher.watch(scope, tx.clone()).unwrap();
        let readkey_cancel = readkey_thread(scope, term.clone(), tx);

        while let Ok(msg) = rx.recv() {
            match msg {
                WatcherMessage::FileComplete(file_path) => {
                    let file_name = file_path.file_name().unwrap().to_string_lossy();
                    let file = downloads.iter()
                        .find(|d| d.file_name == file_name)
                        .unwrap();
                    instance.install_file(file, &file_path)?;
                    print_download_state(&term, &watcher, &downloads)?;
                },
                WatcherMessage::AllComplete => {
                    break;
                },
                WatcherMessage::KeyPress(ch) => {
                    match ch {
                        'o' => {
                            open_urls(
                                downloads.iter()
                                    .filter_map(|d| match watcher.is_file_complete(&d.file_name) {
                                        false => Some(d.url.as_str()),
                                        _ => None
                                    })
                            )?;
                        },
                        'x' => {
                            break;
                        },
                        _ => { }
                    }
                },
                WatcherMessage::Error(_) => {
                    break;
                }
            }
        }

        watch_cancel();
        readkey_cancel();

        term.clear_to_end_of_screen()?;
        term.show_cursor()?;

        Ok(())
    })?;

    Ok(())
}

fn print_download_state(term: &Term, watcher: &DownloadWatcher, downloads: &Vec<FileDownload>) -> IoResult<()> {
    for x in downloads {
        let status = match watcher.is_file_complete(&x.file_name) {
            true => "✅", false => "❌"
        };

        term.write_line(
            format!("{status} {url}", url = x.url).as_str()
        )?;
    }

    term.move_cursor_up(downloads.len())?;

    Ok(())
}

fn open_urls<'a, T>(urls: T) -> IoResult<()>
    where T: Iterator<Item = &'a str>
{
    for u in urls {
        Command::new("xdg-open")
            .arg(u)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }

    Ok(())
}

fn readkey_thread<'scope, 'env>(scope: &'scope Scope<'scope, 'env>, term: Term, tx: Sender<WatcherMessage>) -> impl Fn() {
    let stop = Arc::new(AtomicBool::new(false));

    let stop_thread = stop.clone();
    let exit_thread = move || stop.store(true, Ordering::Relaxed);

    scope.spawn(move || -> IoResult<()> {
        while !stop_thread.load(Ordering::Relaxed) {
            let ch = term.read_char()?;
            tx.send(WatcherMessage::KeyPress(ch)).unwrap();
        }

        Ok(())
    });

    exit_thread
}

fn format_modpack_results<'a, I>(items: I) -> Vec<String>
    where I: Iterator<Item = &'a ModpackManifest>
{
    items.map(|m| format!(
        "{}\n  by: {}",
        m.name,
        m.authors.first().map_or("", |a| &a.name)
    )).collect()
}

fn format_modpack_versions<'a, I>(items: I) -> Vec<String>
    where I: Iterator<Item = &'a ModpackVersion>
{
    items.map(|v| v.name.clone()).collect()
}
