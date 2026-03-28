use color_eyre::eyre::{eyre, Context, Result};
use directories::ProjectDirs;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root_dir: PathBuf,
    pub keys_dir: PathBuf,
    pub db_dir: PathBuf,
    pub db_file: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        let root_dir = if let Ok(value) = env::var("IGARA_CONFIG_DIR") {
            PathBuf::from(value)
        } else {
            let dirs = ProjectDirs::from("com", "lucas-eufrasio", "igara")
                .ok_or_else(|| eyre!("failed to resolve config directory"))?;
            dirs.config_dir().to_path_buf()
        };

        let keys_dir = root_dir.join("keys");
        let db_dir = root_dir.join("db");
        let logs_dir = root_dir.join("logs");
        let db_file = db_dir.join("igara.sqlite3");

        Ok(Self {
            root_dir,
            keys_dir,
            db_dir,
            db_file,
            logs_dir,
        })
    }

    pub fn ensure(&self) -> Result<()> {
        for dir in [&self.root_dir, &self.keys_dir, &self.db_dir, &self.logs_dir] {
            fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
        }
        Ok(())
    }

    pub fn ssh_config_path(&self) -> Result<PathBuf> {
        let home = env::var("HOME").context("HOME environment variable is not set")?;
        let ssh_dir = PathBuf::from(home).join(".ssh");
        fs::create_dir_all(&ssh_dir)?;
        Ok(ssh_dir.join("config"))
    }
}
