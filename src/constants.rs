use std::path::PathBuf;

use crate::error::{ErrorKind, Result};

pub fn pulse_directory() -> Result<PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| ErrorKind::NoHomeDirectory.into())
        .map(|mut home| {
            home.push(".pulse");
            home
        })
}
