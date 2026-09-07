use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::PathBuf};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub workspace: String,
    pub starred: HashSet<String>,
    pub read: HashSet<String>,
    pub archived: HashSet<String>,
    pub notes: String,
    pub light_mode: bool,
}

fn path(user: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        !user.is_empty() && user.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "Invalid preference identity."
    );
    if let Some(base) = std::env::var_os("ASANA_GPUI_DATA_DIR") {
        let base = PathBuf::from(base);
        anyhow::ensure!(
            base.is_absolute(),
            "ASANA_GPUI_DATA_DIR must be an absolute path."
        );
        return Ok(base.join(format!("{user}.json")));
    }
    let base = if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME").context("Home directory is unavailable.")?)
            .join("Library/Application Support")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(
            std::env::var_os("APPDATA").context("Application data directory is unavailable.")?,
        )
    } else {
        match std::env::var_os("XDG_CONFIG_HOME") {
            Some(base) => PathBuf::from(base),
            None => {
                PathBuf::from(std::env::var_os("HOME").context("Home directory is unavailable.")?)
                    .join(".config")
            }
        }
    };
    anyhow::ensure!(
        base.is_absolute(),
        "The application data directory must be an absolute path."
    );
    Ok(base.join("asana-gpui").join(format!("{user}.json")))
}

impl Preferences {
    pub fn load(user: &str) -> Result<Self> {
        let path = path(user)?;
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .context("Local preferences could not be read. The file was left untouched."),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).context("Could not read local preferences."),
        }
    }
    pub fn save(&self, user: &str) -> Result<()> {
        let path = path(user)?;
        let parent = path.parent().context("Missing preferences directory.")?;
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
        let temp = path.with_extension("json.tmp");
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        use std::io::Write;
        let mut file = options.open(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(self)?)?;
        file.sync_all()?;
        fs::rename(temp, path).context("Could not save local preferences.")
    }
}
