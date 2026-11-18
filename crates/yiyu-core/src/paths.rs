use crate::{YiyuError, YiyuResult};
use std::path::{Path, PathBuf};

pub const APP_IDENTIFIER: &str = "com.yiyu.desktop";
pub const APP_NAME: &str = "Yiyu";
pub const DATA_DIR_ENV: &str = "YIYU_DATA_DIR";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    pub fn standard() -> YiyuResult<Self> {
        if let Some(value) = std::env::var_os(DATA_DIR_ENV).filter(|value| !value.is_empty()) {
            return Ok(Self::new(PathBuf::from(value)));
        }

        #[cfg(target_os = "windows")]
        let base = dirs::data_local_dir().ok_or_else(|| {
            YiyuError::storage("cannot resolve the local application data directory")
        })?;

        #[cfg(not(target_os = "windows"))]
        let base = dirs::data_dir()
            .ok_or_else(|| YiyuError::storage("cannot resolve the application data directory"))?;

        Ok(Self::new(base.join(APP_NAME)))
    }

    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn history_file(&self) -> PathBuf {
        self.root.join("history.sqlite")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn updates_dir(&self) -> PathBuf {
        self.root.join("updates")
    }

    pub fn ensure(&self) -> YiyuResult<()> {
        for directory in [self.root.clone(), self.logs_dir(), self.updates_dir()] {
            std::fs::create_dir_all(&directory)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_runtime_files_are_inside_root() {
        let paths = AppPaths::new("/tmp/yiyu-test");
        assert!(paths.settings_file().starts_with(paths.root()));
        assert!(paths.history_file().starts_with(paths.root()));
        assert!(paths.logs_dir().starts_with(paths.root()));
        assert!(paths.updates_dir().starts_with(paths.root()));
    }

    #[test]
    fn ensure_creates_private_directories() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        assert!(paths.logs_dir().is_dir());
        assert!(paths.updates_dir().is_dir());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(paths.root())
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o700);
        }
    }
}
