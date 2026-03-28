use color_eyre::eyre::{Context, Result, eyre};
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
            fs::create_dir_all(dir)
                .with_context(|| format!("failed to create {}", dir.display()))?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.keys_dir, fs::Permissions::from_mode(0o700))
                .with_context(|| format!("failed to set 0700 permissions on {}", self.keys_dir.display()))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_app_paths_env_and_ensure() -> Result<()> {
        let dir = tempdir()?;
        let path_str = dir.path().to_str().unwrap();
        
        // Use a single test to avoid race conditions with environment variables
        unsafe {
            env::set_var("IGARA_CONFIG_DIR", path_str);
        }

        let paths = AppPaths::resolve()?;
        assert_eq!(paths.root_dir, dir.path());
        assert!(paths.db_file.to_str().unwrap().contains("igara.sqlite3"));

        paths.ensure()?;
        assert!(paths.keys_dir.exists());
        assert!(paths.db_dir.exists());
        assert!(paths.logs_dir.exists());

        unsafe {
            env::remove_var("IGARA_CONFIG_DIR");
        }
        Ok(())
    }
}
