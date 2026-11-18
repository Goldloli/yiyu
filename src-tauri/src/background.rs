//! Locally managed background images. The renderer never supplies a filesystem path.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{ImageFormat, ImageReader, Limits};
use std::fs::{self, OpenOptions};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yiyu_core::settings::{SettingsPatch, SettingsService, SettingsSnapshot};
use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

const MAX_BYTES: usize = 12 * 1024 * 1024;
const MAX_DIMENSION: u32 = 8192;
const MAX_PIXELS: u64 = 32_000_000;

fn invalid(message: &str) -> YiyuError {
    YiyuError::new(ErrorKind::Validation, message).field("background")
}

fn validate(bytes: &[u8]) -> YiyuResult<&'static str> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(invalid("Background image must be no larger than 12 MB"));
    }
    let format =
        image::guess_format(bytes).map_err(|_| invalid("Choose a valid PNG or JPEG image"))?;
    let mime = match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        _ => return Err(invalid("Choose a valid PNG or JPEG image")),
    };
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| invalid("The image cannot be decoded"))?;
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        return Err(invalid("Image dimensions exceed the supported limit"));
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(160 * 1024 * 1024);
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|_| invalid("The image cannot be decoded"))?;
    Ok(mime)
}

fn managed_path(root: &Path, id: &str) -> YiyuResult<PathBuf> {
    let id = Uuid::parse_str(id).map_err(|_| invalid("Invalid background identifier"))?;
    Ok(root.join("backgrounds").join(format!("{id}.image")))
}

pub fn stage(root: &Path, data: &str) -> YiyuResult<String> {
    if data.len() > MAX_BYTES.div_ceil(3) * 4 + 4 {
        return Err(invalid("Background image must be no larger than 12 MB"));
    }
    let bytes = STANDARD
        .decode(data)
        .map_err(|_| invalid("The image cannot be decoded"))?;
    validate(&bytes)?;
    let directory = root.join("backgrounds");
    fs::create_dir_all(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    }
    let id = Uuid::new_v4().to_string();
    let path = managed_path(root, &id)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| -> std::io::Result<()> {
        let mut file = options.open(&path)?;
        file.write_all(&bytes)?;
        file.sync_all()
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(path);
        return Err(error.into());
    }
    Ok(id)
}

pub fn read(root: &Path, id: &str) -> YiyuResult<Option<String>> {
    let path = managed_path(root, id)?;
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mime = match validate(&bytes) {
        Ok(mime) => mime,
        Err(_) => return Ok(None),
    };
    Ok(Some(format!(
        "data:{mime};base64,{}",
        STANDARD.encode(bytes)
    )))
}

pub fn remove(root: &Path, id: &str) {
    if let Ok(path) = managed_path(root, id) {
        let _ = fs::remove_file(path);
    }
}

pub struct BackgroundManager {
    root: PathBuf,
    commit: tokio::sync::Mutex<()>,
}

impl BackgroundManager {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            commit: tokio::sync::Mutex::new(()),
        }
    }

    pub async fn current(&self, settings: &SettingsService) -> YiyuResult<Option<String>> {
        let _guard = self.commit.lock().await;
        match settings.snapshot().await.settings.background_id.as_deref() {
            Some(id) => read(&self.root, id),
            None => Ok(None),
        }
    }

    pub async fn replace(
        &self,
        settings: &SettingsService,
        expected_revision: u64,
        data: Option<&str>,
    ) -> YiyuResult<SettingsSnapshot> {
        let _guard = self.commit.lock().await;
        let previous = settings.snapshot().await;
        if expected_revision != previous.revision {
            return Err(invalid("Settings changed; try again").field("revision"));
        }
        let candidate = match data {
            Some(data) => Some(stage(&self.root, data)?),
            None => None,
        };
        let result = settings
            .patch(
                expected_revision,
                SettingsPatch {
                    background_id: Some(candidate.clone()),
                    background_choice: Some(
                        if candidate.is_some() {
                            "custom"
                        } else {
                            "none"
                        }
                        .into(),
                    ),
                    ..Default::default()
                },
            )
            .await;
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(error) => {
                if let Some(id) = candidate {
                    remove(&self.root, &id);
                }
                return Err(error);
            }
        };
        if let Some(old) = previous.settings.background_id {
            remove(&self.root, &old);
        }
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(format: ImageFormat) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(4, 3)
            .write_to(&mut bytes, format)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn validates_content_and_bounds() {
        assert_eq!(validate(&image(ImageFormat::Png)).unwrap(), "image/png");
        assert_eq!(validate(&image(ImageFormat::Jpeg)).unwrap(), "image/jpeg");
        assert!(validate(b"not an image").is_err());
        assert!(validate(&image(ImageFormat::Png)[..20]).is_err());
        assert!(validate(&vec![0; MAX_BYTES + 1]).is_err());
        let mut oversized = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(MAX_DIMENSION + 1, 1)
            .write_to(&mut oversized, ImageFormat::Png)
            .unwrap();
        assert!(validate(oversized.get_ref()).is_err());
        assert!(managed_path(Path::new("/tmp"), "../settings.json").is_err());
    }

    #[test]
    fn staged_image_is_private_and_removal_is_limited() {
        let temp = tempfile::tempdir().unwrap();
        let unrelated = temp.path().join("keep.txt");
        fs::write(&unrelated, "keep").unwrap();
        let id = stage(temp.path(), &STANDARD.encode(image(ImageFormat::Png))).unwrap();
        assert!(
            read(temp.path(), &id)
                .unwrap()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        remove(temp.path(), &id);
        assert!(read(temp.path(), &id).unwrap().is_none());
        remove(temp.path(), "../keep.txt");
        assert_eq!(fs::read_to_string(unrelated).unwrap(), "keep");
    }

    #[tokio::test]
    async fn replacement_conflicts_and_failed_persistence_keep_the_previous_image() {
        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path());
        paths.ensure().unwrap();
        let settings = SettingsService::new(&paths).unwrap();
        let initial = settings
            .patch(
                0,
                SettingsPatch {
                    active_provider_id: Some(Some("openai".into())),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let manager = BackgroundManager::new(temp.path());
        let png = STANDARD.encode(image(ImageFormat::Png));
        let first = manager
            .replace(&settings, initial.revision, Some(&png))
            .await
            .unwrap();
        let (a, b) = tokio::join!(
            manager.replace(&settings, first.revision, Some(&png)),
            manager.replace(&settings, first.revision, Some(&png))
        );
        assert_ne!(a.is_ok(), b.is_ok());
        let missing = settings.snapshot().await;
        assert!(manager.current(&settings).await.unwrap().is_some());
        remove(
            temp.path(),
            missing.settings.background_id.as_deref().unwrap(),
        );
        assert!(manager.current(&settings).await.unwrap().is_none());
        let current = manager
            .replace(&settings, missing.revision, Some(&png))
            .await
            .unwrap();
        assert!(manager.current(&settings).await.unwrap().is_some());
        fs::remove_file(paths.settings_file()).unwrap();
        fs::create_dir(paths.settings_file()).unwrap();
        assert!(
            manager
                .replace(&settings, current.revision, Some(&png))
                .await
                .is_err()
        );
        assert_eq!(settings.snapshot().await, current);
        assert!(manager.current(&settings).await.unwrap().is_some());
        assert_eq!(
            fs::read_dir(temp.path().join("backgrounds"))
                .unwrap()
                .count(),
            1
        );
    }
}
