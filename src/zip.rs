use std::{fs::{self, File}, io::{self, Result, Read, Seek, Write}, path::Path};
use walkdir::{DirEntry, WalkDir};
use zip::{result::ZipResult, write::SimpleFileOptions, ZipArchive, ZipWriter};

// extract/create adapted from examples here
// https://github.com/zip-rs/zip/tree/21a20584bc9e05dfa4f3c5b0bc420a1389fae2c3/examples

pub fn extract_zip(zip_file: File, out_dir: &Path) -> Result<()> {
    let mut archive = ZipArchive::new(zip_file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => out_dir.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

fn create_zip(zip_file: File, src_dir: &Path) -> Result<()> {
    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();

    zip_dir(&mut it.filter_map(|e| e.ok()), src_dir, zip_file)?;

    Ok(())
}

fn zip_dir<T>(
    it: &mut dyn Iterator<Item = DirEntry>,
    src_dir: &Path,
    writer: T
) -> ZipResult<()>
    where T: Write + Seek
{
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default();

    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(src_dir).unwrap();

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            zip.start_file_from_path(name, options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            zip.add_directory_from_path(name, options)?;
        }
    }

    zip.finish()?;

    ZipResult::Ok(())
}

pub fn make_modded_jar<P, I>(output_jar: P, mc_jar: P, jar_mods: I) -> Result<()>
    where P: AsRef<Path>, I: Iterator, I::Item: AsRef<Path>
{
    let zip_temp_dir = std::env::temp_dir().join("minecraft_jar");
    if zip_temp_dir.exists() {
        fs::remove_dir_all(&zip_temp_dir)?;
    }

    fs::create_dir_all(&zip_temp_dir)?;

    // first, extract the vanilla MC jar
    extract_zip(fs::File::open(mc_jar)?, &zip_temp_dir)?;

    // I remember doing this a lot when creating modded MC jar
    // not sure if it's strictly required
    fs::remove_dir_all(zip_temp_dir.join("META-INF"))?;

    // extract all the jar mods overtop of the vanilla files
    for jar_path in jar_mods {
        extract_zip(fs::File::open(jar_path)?, &zip_temp_dir)?;
    }

    create_zip(fs::File::create(&output_jar)?, &zip_temp_dir)?;

    Ok(())
}
