//! Where panosync keeps its files, and the config file itself.
//!
//! The config directory is `<OS config dir>/panosync` (`~/.config/panosync` on Linux,
//! `~/Library/Application Support/panosync` on macOS, `%APPDATA%\panosync` on Windows),
//! unless the `PANOSYNC_CONFIG_DIR` environment variable points somewhere else.
//! It holds `config.toml` (relay address and key) and the `pause` state file.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Overrides the config directory, e.g. to run two clients on one machine.
pub const CONFIG_DIR_ENV: &str = "PANOSYNC_CONFIG_DIR";

pub struct ConfigDir(PathBuf);

impl ConfigDir {
    pub fn locate() -> Result<Self> {
        if let Some(dir) = std::env::var_os(CONFIG_DIR_ENV).filter(|d| !d.is_empty()) {
            return Ok(Self(dir.into()));
        }
        let base = dirs::config_dir().context("could not find the config directory")?;
        Ok(Self(base.join("panosync")))
    }

    pub fn config_file(&self) -> PathBuf {
        self.0.join("config.toml")
    }

    pub fn pause_file(&self) -> PathBuf {
        self.0.join(panosync::pause::FILE_NAME)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Relay address, e.g. `wss://panosync.example.com`.
    pub server: String,
    /// The shared master key.
    pub key: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path).with_context(|| {
            format!(
                "config not found ({}). Run `panosync setup` first.",
                path.display()
            )
        })?;
        toml::from_str(&contents).with_context(|| format!("could not parse {}", path.display()))
    }

    /// Saves the config. On Unix the file is readable only by the current user, because it
    /// contains the key. On Windows it inherits the permissions of the user's profile.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        write_private(path, toml::to_string(self)?.as_bytes())
            .with_context(|| format!("could not write {}", path.display()))
    }
}

#[cfg(unix)]
fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt, os::unix::fs::PermissionsExt};
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    // `mode` only applies when the file is created; fix up an existing file too.
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(contents)
}

#[cfg(not(unix))]
fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, contents)
}
