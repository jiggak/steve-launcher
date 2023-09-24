use std::{fs, io, path::Path};

/// Copy all files recursively from the source directory to destination directory
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
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
