use dirs::home_dir;
use std::path::{Path, PathBuf};

pub fn expand_home_dir<P: AsRef<Path> + ?Sized>(path: &P) -> PathBuf {
    let path = path.as_ref();

    if !path.starts_with("~") {
        return path.into();
    }

    home_dir()
        .unwrap()
        .join(path.strip_prefix("~").unwrap())
        .into()
}
