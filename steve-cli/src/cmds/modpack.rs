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

use anyhow::{Result, anyhow};
use console::Term;
use dialoguer::Select;
use std::{
    io::Result as IoResult, path::Path, process::{Command, Stdio},
    sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{self, Sender}},
    thread::{self, Scope}
};

use crate::ProgressBars;
use steve::{
    BeginProgress, CurseForgeZip, DownloadWatcher, FileDownload, Installer,
    Instance, Modpack, ModpackId, ModpackManifest, ModpackVersion,
    ModpackVersionManifest, ModpacksClient, WatcherMessage
};
use super::{console_theme, prompt_confirm};

pub async fn modpack_search_and_install(
    instance_dir: &Path,
    search: &str,
    limit: u8
) -> Result<()> {
    let pack = search_modpacks(search, limit).await?;

    let mut instance = if Instance::exists(instance_dir) {
        if !prompt_confirm("Instance already exists, are you sure you want to install the pack here?")? {
            return Ok(())
        }

        let mut instance = Instance::load(instance_dir)?;

        instance.set_mc_version(pack.get_minecraft_version()?)?;
        instance.set_mod_loader(pack.get_mod_loader()?)?;

        instance
    } else {
        Instance::create(
            instance_dir,
            &pack.get_minecraft_version()?,
            pack.get_mod_loader()?
        ).await?
    };

    install_pack(&mut instance, false, &pack).await?;

    Ok(())
}

pub async fn search_modpacks(search: &str, limit: u8) -> Result<ModpackVersionManifest> {
    let progress = ProgressBars::new();
    let client = ModpacksClient::new();

    let results = client.search_modpacks(search, limit).await?;

    let mut search_results = Vec::new();

    let progress = progress.begin("Retrieving search results", results.total as usize);
    let mut count:usize = 0;

    for pack_id in results.pack_ids {
        search_results.push(
            client.get_ftb_modpack_versions(pack_id).await?
        );

        count += 1;
        progress.set_position(count);
    }

    for curse_id in results.curseforge_ids {
        search_results.push(
            client.get_curse_modpack_versions(curse_id).await?
        );

        count += 1;
        progress.set_position(count);
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

    let pack = if selected_pack.provider == "curseforge" {
        client.get_curse_modpack(selected_pack.pack_id, selected_version.version_id).await?
    } else {
        client.get_ftb_modpack(selected_pack.pack_id, selected_version.version_id).await?
    };

    Ok(pack)
}

pub async fn get_ftb_pack(pack_id: u32) -> Result<ModpackVersionManifest> {
    let client = ModpacksClient::new();

    let manifest = client.get_ftb_modpack_versions(pack_id).await?;

    let selection = Select::with_theme(&console_theme())
        .with_prompt("Select modpack version")
        .items(&format_modpack_versions(manifest.versions.iter()))
        .default(0)
        .interact()?;

    let version = &manifest.versions[selection];

    Ok(client.get_ftb_modpack(pack_id, version.version_id).await?)
}

pub async fn install_pack(
    instance: &mut Instance,
    is_server: bool,
    pack: &ModpackVersionManifest
) -> Result<()> {
    let dest_dir = instance.game_dir();
    let progress = ProgressBars::new();
    let installer = Installer::new(&dest_dir);

    let (pack_files, downloads) = installer.install_pack(pack, is_server, &progress)
        .await?;

    if let Some(downloads) = downloads {
        download_blocked(&installer, downloads)?;
    }

    instance.remove_old_modpack_files(&pack_files)?;

    let pack_files: Vec<_> = pack_files.iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect();

    instance.set_modpack_manifest(
        Modpack {
            id: ModpackId::CurseForge {
                mod_id: pack.pack_id,
                version: pack.name.to_string()
            },
            files: pack_files
        }
    )?;

    Ok(())
}

pub async fn modpack_zip_install(
    instance_dir: &Path,
    zip_file: &Path
) -> Result<()> {
    let progress = ProgressBars::new();

    let pack = CurseForgeZip::load_zip(zip_file)?;

    let mut instance = if Instance::exists(instance_dir) {
        if !prompt_confirm("Instance already exists, are you sure you want to install the pack here?")? {
            return Ok(())
        }

        let mut instance = Instance::load(instance_dir)?;

        instance.set_mc_version(pack.manifest.minecraft.version.clone())?;
        instance.set_mod_loader(pack.manifest.minecraft.get_mod_loader()?)?;

        instance
    } else {
        Instance::create(
            instance_dir,
            &pack.manifest.minecraft.version,
            pack.manifest.minecraft.get_mod_loader()?
        ).await?
    };

    let installer = Installer::new(&instance.game_dir());
    let (pack_files, downloads) = installer.install_pack_zip(&pack, &progress)
        .await?;

    if let Some(downloads) = downloads {
        download_blocked(&installer, downloads)?;
    }

    instance.remove_old_modpack_files(&pack_files)?;

    let pack_files: Vec<_> = pack_files.iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect();

    instance.set_modpack_manifest(
        Modpack {
            id: ModpackId::CurseZip {
                file_name: zip_file.file_name().unwrap().to_string_lossy().to_string()
            },
            files: pack_files
         }
    )?;

    Ok(())
}

pub async fn modpack_update(instance_dir: &Path) -> Result<()> {
    let mut instance = Instance::load(instance_dir)?;

    if let Some(modpack) = &instance.manifest.modpack {
        if let ModpackId::CurseForge { mod_id, version } = &modpack.id {
            let client = ModpacksClient::new();

            let pack_id = *mod_id;
            let pack = client.get_curse_modpack_versions(pack_id)
                .await?;

            // hopefully it's safe to assume first version is latests
            let latest = pack.versions.first()
                .ok_or(anyhow!("Pack data has empty `versions` list"))?;

            println!("Current: {version}");
            println!("Latest: {}", latest.name);

            if prompt_confirm("Would you like to (re)install latest version?")? {
                let pack = client.get_curse_modpack(pack_id, latest.version_id)
                    .await?;

                install_pack(&mut instance, false, &pack)
                    .await?;
            }
        } else {
            println!("Only CurseForge instances can be updates automatically");
        }
    }

    Ok(())
}

fn download_blocked(installer: &Installer, downloads: Vec<FileDownload>) -> Result<()> {
    let watcher = DownloadWatcher::new(
        downloads.iter()
            .map(|f| f.file_name.as_str())
    );

    // copy any downloads already in watch dir
    for f in &downloads {
        if watcher.is_file_complete(&f.file_name) {
            let file_path = watcher.watch_dir.join(&f.file_name);
            installer.install_file(f, &file_path)?;
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

    thread::scope(|scope| -> Result<()> {
        let watch_cancel = watcher.watch(scope, tx.clone())?;
        let readkey_cancel = readkey_thread(scope, term.clone(), tx);

        while let Ok(msg) = rx.recv() {
            match msg {
                WatcherMessage::FileComplete(file_path) => {
                    let file_name = file_path.file_name().unwrap().to_string_lossy();
                    let file = downloads.iter()
                        .find(|d| d.file_name == file_name)
                        .unwrap();
                    installer.install_file(file, &file_path)?;
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

fn readkey_thread<'scope>(scope: &'scope Scope<'scope, '_>, term: Term, tx: Sender<WatcherMessage>) -> impl Fn() {
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
