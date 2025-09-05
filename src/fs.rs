use std::{fs, io};
use std::path::{Path, PathBuf};

/// Copy all files recursively from the source directory to destination directory
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }

    Ok(())
}

/// Iterate and copy `src_files` to `dst` directory
pub fn copy_files<I, P>(src_files: I, dst: P) -> io::Result<()>
    where I: Iterator, I::Item: AsRef<Path>, P: AsRef<Path>
{
    fs::create_dir_all(&dst)?;

    for file in src_files {
        fs::copy(file.as_ref(), dst.as_ref().join(file.as_ref().file_name().unwrap()))?;
    }

    Ok(())
}

pub fn list_files_in_dir<P: AsRef<Path>>(dir: P) -> io::Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    let mut files = Vec::new();

    fn visit_dir(base: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;

            if file_type.is_file() {
                if let Ok(rel_path) = path.strip_prefix(base) {
                    files.push(rel_path.to_path_buf());
                }
            } else if file_type.is_dir() {
                visit_dir(base, &path, files)?;
            }
        }

        Ok(())
    }

    match fs::metadata(dir) {
        Ok(meta) if meta.is_dir() => visit_dir(dir, dir, &mut files)?,
        Ok(_) => return Err(io::Error::other(format!("Not a directory: {:?}", dir.file_name()))),
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e)
    }

    Ok(files)
}
