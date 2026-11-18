use crate::dto::AppInfoDTO;
use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use yiyu_core::settings::ProxyConfig;
use yiyu_core::transport::build_client;
use yiyu_core::update::{
    ReleaseInfo, UpdatePlatform, expected_sha256, is_newer, parse_github_release, parse_sha256_sums,
};
use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

const RELEASE_URL_ENV: &str = "YIYU_UPDATE_RELEASE_URL";
const DEFAULT_RELEASE_URL: &str = "https://api.github.com/repos/Goldloli/yiyu/releases/latest";
const AUTO_CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const LAST_CHECK_FILE: &str = "last-update-check";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshotDTO {
    pub status: String,
    pub current_version: String,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub asset_name: Option<String>,
    pub file_path: Option<String>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub progress: Option<f64>,
    pub bytes_per_second: Option<u64>,
    pub eta_seconds: Option<u64>,
    pub error: Option<YiyuError>,
}

impl UpdateSnapshotDTO {
    fn idle(current_version: String) -> Self {
        Self {
            status: "idle".into(),
            current_version,
            version: None,
            notes: None,
            asset_name: None,
            file_path: None,
            downloaded_bytes: 0,
            total_bytes: None,
            progress: None,
            bytes_per_second: None,
            eta_seconds: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedUpdate {
    release: ReleaseInfo,
}

#[derive(Debug, Clone)]
struct VerifiedUpdate {
    path: PathBuf,
    sha256: String,
}

pub struct UpdateManager {
    paths: yiyu_core::paths::AppPaths,
    snapshot: Mutex<UpdateSnapshotDTO>,
    resolved: Mutex<Option<ResolvedUpdate>>,
    verified: Mutex<Option<VerifiedUpdate>>,
    cancel: Mutex<CancellationToken>,
    operation: tokio::sync::Mutex<()>,
}

impl UpdateManager {
    pub fn new(paths: yiyu_core::paths::AppPaths) -> Self {
        Self {
            paths,
            snapshot: Mutex::new(UpdateSnapshotDTO::idle(
                env!("CARGO_PKG_VERSION").to_string(),
            )),
            resolved: Mutex::new(None),
            verified: Mutex::new(None),
            cancel: Mutex::new(CancellationToken::new()),
            operation: tokio::sync::Mutex::new(()),
        }
    }

    pub fn snapshot(&self) -> UpdateSnapshotDTO {
        self.snapshot.lock().expect("update state poisoned").clone()
    }

    pub fn automatic_check_due(&self) -> bool {
        automatic_check_due_at(&self.last_check_path(), unix_now())
    }

    fn last_check_path(&self) -> PathBuf {
        self.paths.root().join(LAST_CHECK_FILE)
    }

    fn record_successful_check(&self) {
        if let Some(parent) = self.last_check_path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(self.last_check_path(), unix_now().to_string());
    }

    fn set_snapshot(&self, snapshot: UpdateSnapshotDTO) -> UpdateSnapshotDTO {
        *self.snapshot.lock().expect("update state poisoned") = snapshot.clone();
        snapshot
    }

    pub async fn check(&self, proxy: &ProxyConfig, force: bool) -> YiyuResult<UpdateSnapshotDTO> {
        let _operation = self.operation.try_lock().map_err(|_| {
            YiyuError::new(ErrorKind::Update, "another update operation is in progress")
        })?;
        let current = env!("CARGO_PKG_VERSION");
        {
            let snapshot = self.snapshot();
            if !force && snapshot.status == "available" {
                return Ok(snapshot);
            }
        }
        *self.verified.lock().expect("update state poisoned") = None;
        *self.resolved.lock().expect("update state poisoned") = None;
        let mut checking = UpdateSnapshotDTO::idle(current.into());
        checking.status = "checking".into();
        self.set_snapshot(checking);

        let result = self.fetch_release(proxy, current).await;
        match result {
            Ok(snapshot) => Ok(self.set_snapshot(snapshot)),
            Err(error) => {
                let mut failed = UpdateSnapshotDTO::idle(current.into());
                failed.status = "error".into();
                failed.error = Some(error.clone());
                self.set_snapshot(failed);
                Err(error)
            }
        }
    }

    async fn fetch_release(
        &self,
        proxy: &ProxyConfig,
        current: &str,
    ) -> YiyuResult<UpdateSnapshotDTO> {
        let url = url::Url::parse(
            std::env::var(RELEASE_URL_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .as_deref()
                .unwrap_or(DEFAULT_RELEASE_URL),
        )?;
        self.fetch_release_from_url(url, proxy, current).await
    }

    async fn fetch_release_from_url(
        &self,
        url: url::Url,
        proxy: &ProxyConfig,
        current: &str,
    ) -> YiyuResult<UpdateSnapshotDTO> {
        let client = build_client(proxy, &url)?;
        let response = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", format!("Yiyu/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .send()
            .await?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            self.record_successful_check();
            let mut snapshot = UpdateSnapshotDTO::idle(current.into());
            snapshot.status = "noRelease".into();
            return Ok(snapshot);
        }
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(YiyuError::new(
                ErrorKind::Update,
                format!("update metadata request failed with HTTP {status}"),
            )
            .retryable(true));
        }
        let platform = current_platform()?;
        let release = parse_github_release(&body, platform)?;
        let newer = is_newer(current, &release.version)?;
        self.record_successful_check();
        if !newer {
            let mut snapshot = UpdateSnapshotDTO::idle(current.into());
            snapshot.status = "upToDate".into();
            return Ok(snapshot);
        }
        *self.resolved.lock().expect("update state poisoned") = Some(ResolvedUpdate {
            release: release.clone(),
        });
        let mut snapshot = UpdateSnapshotDTO::idle(current.into());
        snapshot.status = "available".into();
        snapshot.version = Some(release.version);
        snapshot.notes = Some(release.body);
        snapshot.asset_name = Some(release.installer.name);
        snapshot.total_bytes = Some(release.installer.size);
        Ok(snapshot)
    }

    pub async fn download<F>(
        &self,
        proxy: &ProxyConfig,
        mut on_progress: F,
    ) -> YiyuResult<UpdateSnapshotDTO>
    where
        F: FnMut(&UpdateSnapshotDTO),
    {
        let _operation = self.operation.try_lock().map_err(|_| {
            YiyuError::new(ErrorKind::Update, "another update operation is in progress")
        })?;
        let cancel = {
            let mut token = self.cancel.lock().expect("update state poisoned");
            *token = CancellationToken::new();
            token.clone()
        };
        let result = self.download_inner(proxy, &cancel, &mut on_progress).await;
        if let Err(error) = &result {
            *self.verified.lock().expect("update state poisoned") = None;
            let mut failed = self.snapshot();
            failed.status = "error".into();
            failed.error = Some(error.clone());
            self.set_snapshot(failed.clone());
            on_progress(&failed);
        }
        result
    }

    async fn download_inner<F>(
        &self,
        proxy: &ProxyConfig,
        cancel: &CancellationToken,
        on_progress: &mut F,
    ) -> YiyuResult<UpdateSnapshotDTO>
    where
        F: FnMut(&UpdateSnapshotDTO),
    {
        let resolved = self
            .resolved
            .lock()
            .expect("update state poisoned")
            .clone()
            .ok_or_else(|| {
                YiyuError::new(ErrorKind::Update, "check for updates before downloading")
            })?;
        *self.verified.lock().expect("update state poisoned") = None;
        self.paths.ensure()?;
        let asset = resolved.release.installer.clone();
        let safe_name = safe_asset_name(&asset.name)?;
        let final_path = self.paths.updates_dir().join(&safe_name);
        let part_path = part_file_path(&final_path);
        let existing = tokio::fs::metadata(&part_path)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);

        let mut snapshot = self.snapshot();
        snapshot.status = "downloading".into();
        snapshot.downloaded_bytes = existing;
        snapshot.total_bytes = Some(asset.size);
        snapshot.progress = progress(existing, asset.size);
        snapshot.bytes_per_second = None;
        snapshot.eta_seconds = None;
        snapshot.file_path = None;
        snapshot.error = None;
        self.set_snapshot(snapshot.clone());
        on_progress(&snapshot);

        let mut downloaded = existing;
        let transfer_started = Instant::now();
        macro_rules! cancellable {
            ($future:expr) => {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        snapshot.status = "cancelled".into();
                        snapshot.downloaded_bytes = downloaded;
                        snapshot.progress = progress(downloaded, asset.size);
                        snapshot.bytes_per_second = None;
                        snapshot.eta_seconds = None;
                        self.set_snapshot(snapshot.clone());
                        on_progress(&snapshot);
                        return Ok(snapshot);
                    }
                    value = $future => value,
                }
            };
        }

        if asset.size == 0 || existing != asset.size {
            let installer_url = url::Url::parse(&asset.browser_download_url)?;
            let client = build_client(proxy, &installer_url)?;
            let resume_from = if asset.size > 0 && existing > asset.size {
                0
            } else {
                existing
            };
            let mut request = client.get(installer_url.clone());
            if resume_from > 0 {
                request = request.header("Range", format!("bytes={resume_from}-"));
            }
            let mut response = cancellable!(request.send())?;
            let mut restarted = false;
            if resume_from > 0 && response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
                response = cancellable!(client.get(installer_url).send())?;
                restarted = true;
            }
            let status = response.status();
            let append =
                resume_from > 0 && !restarted && status == reqwest::StatusCode::PARTIAL_CONTENT;
            if !status.is_success() || (status == reqwest::StatusCode::PARTIAL_CONTENT && !append) {
                return Err(YiyuError::new(
                    ErrorKind::Update,
                    format!("update download failed with HTTP {status}"),
                )
                .retryable(true));
            }
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(append)
                .truncate(!append)
                .open(&part_path)
                .await?;
            downloaded = if append { resume_from } else { 0 };
            let transfer_baseline = downloaded;
            let mut stream = response.bytes_stream();
            let mut last_emit = Instant::now();
            loop {
                let Some(chunk) = cancellable!(stream.next()) else {
                    break;
                };
                let bytes = chunk?;
                cancellable!(file.write_all(&bytes))?;
                downloaded += bytes.len() as u64;
                if last_emit.elapsed() >= Duration::from_millis(100) {
                    last_emit = Instant::now();
                    snapshot.downloaded_bytes = downloaded;
                    snapshot.total_bytes = Some(asset.size);
                    snapshot.progress = progress(downloaded, asset.size);
                    apply_transfer_metrics(
                        &mut snapshot,
                        downloaded,
                        asset.size,
                        transfer_baseline,
                        transfer_started,
                    );
                    self.set_snapshot(snapshot.clone());
                    on_progress(&snapshot);
                }
            }
            cancellable!(file.flush())?;
        }

        snapshot.status = "verifying".into();
        snapshot.progress = Some(100.0);
        snapshot.bytes_per_second = None;
        snapshot.eta_seconds = Some(0);
        self.set_snapshot(snapshot.clone());
        on_progress(&snapshot);

        let sums_url = url::Url::parse(&resolved.release.checksums.browser_download_url)?;
        let sums_client = build_client(proxy, &sums_url)?;
        let sums_response = cancellable!(sums_client.get(sums_url).send())?.error_for_status()?;
        let sums_body = cancellable!(sums_response.text())?;
        let sums = parse_sha256_sums(&sums_body);
        let expected = expected_sha256(&sums, &asset.name)?;
        let actual = cancellable!(tokio::task::spawn_blocking({
            let part_path = part_path.clone();
            move || hash_file(&part_path)
        }))
        .map_err(|error| YiyuError::new(ErrorKind::Update, error.to_string()))??;
        if cancel.is_cancelled() {
            snapshot.status = "cancelled".into();
            snapshot.downloaded_bytes = downloaded;
            snapshot.progress = progress(downloaded, asset.size);
            snapshot.bytes_per_second = None;
            snapshot.eta_seconds = None;
            self.set_snapshot(snapshot.clone());
            on_progress(&snapshot);
            return Ok(snapshot);
        }
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&part_path).await;
            return Err(YiyuError::new(
                ErrorKind::Update,
                "downloaded installer checksum does not match SHA256SUMS",
            )
            .retryable(true));
        }
        tokio::fs::rename(&part_path, &final_path).await?;
        *self.verified.lock().expect("update state poisoned") = Some(VerifiedUpdate {
            path: final_path.clone(),
            sha256: actual,
        });
        snapshot.status = "ready".into();
        snapshot.file_path = Some(final_path.display().to_string());
        snapshot.downloaded_bytes = downloaded;
        snapshot.progress = Some(100.0);
        snapshot.bytes_per_second = None;
        snapshot.eta_seconds = Some(0);
        self.set_snapshot(snapshot.clone());
        on_progress(&snapshot);
        Ok(snapshot)
    }

    pub fn cancel(&self) {
        self.cancel.lock().expect("update state poisoned").cancel();
    }

    pub fn verify_ready_file(&self, path: &str) -> YiyuResult<PathBuf> {
        let candidate = PathBuf::from(path);
        let snapshot = self.snapshot();
        let verified = self
            .verified
            .lock()
            .expect("update state poisoned")
            .clone()
            .ok_or_else(|| {
                YiyuError::new(
                    ErrorKind::Installer,
                    "no verified update is ready to install",
                )
            })?;
        let updates_root = std::fs::canonicalize(self.paths.updates_dir()).map_err(|_| {
            YiyuError::new(ErrorKind::Installer, "updates directory is unavailable")
        })?;
        let candidate = std::fs::canonicalize(&candidate).map_err(|_| {
            YiyuError::new(
                ErrorKind::Installer,
                "verified update file no longer exists",
            )
        })?;
        let verified_path = std::fs::canonicalize(&verified.path).map_err(|_| {
            YiyuError::new(
                ErrorKind::Installer,
                "verified update file no longer exists",
            )
        })?;
        if snapshot.status != "ready"
            || candidate != verified_path
            || !candidate.starts_with(&updates_root)
        {
            return Err(YiyuError::new(
                ErrorKind::Installer,
                "installer does not match the verified update",
            ));
        }
        let actual = hash_file(&candidate)?;
        if !actual.eq_ignore_ascii_case(&verified.sha256) {
            return Err(YiyuError::new(
                ErrorKind::Installer,
                "verified update changed after checksum validation",
            ));
        }
        if !candidate.is_file() {
            return Err(YiyuError::new(
                ErrorKind::Installer,
                "verified update file no longer exists",
            ));
        }
        Ok(candidate)
    }
}

pub fn install_ready_update(path: &PathBuf) -> YiyuResult<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|error| {
                YiyuError::new(
                    ErrorKind::Installer,
                    format!("cannot open verified DMG: {error}"),
                )
            })?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new(path).spawn().map_err(|error| {
            YiyuError::new(
                ErrorKind::Installer,
                format!("cannot launch verified installer: {error}"),
            )
        })?;
        Ok(())
    }
}

fn current_platform() -> YiyuResult<UpdatePlatform> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok(UpdatePlatform::MacosAarch64);
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Ok(UpdatePlatform::WindowsX64);
    }
    #[allow(unreachable_code)]
    Err(YiyuError::new(
        ErrorKind::Unsupported,
        "automatic updates are not available for this platform",
    ))
}

fn safe_asset_name(name: &str) -> YiyuResult<String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.contains('\0')
    {
        return Err(YiyuError::new(
            ErrorKind::Update,
            "release asset name contains unsafe path characters",
        ));
    }
    Ok(name.to_string())
}

fn part_file_path(final_path: &Path) -> PathBuf {
    final_path.with_extension(format!(
        "{}part",
        final_path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!("{value}."))
            .unwrap_or_default()
    ))
}

fn progress(done: u64, total: u64) -> Option<f64> {
    (total > 0).then(|| (done as f64 / total as f64 * 100.0).clamp(0.0, 100.0))
}

fn apply_transfer_metrics(
    snapshot: &mut UpdateSnapshotDTO,
    downloaded: u64,
    total: u64,
    baseline: u64,
    started: Instant,
) {
    let transferred = downloaded.saturating_sub(baseline);
    let elapsed = started.elapsed().as_secs_f64();
    if transferred == 0 || elapsed <= 0.0 {
        snapshot.bytes_per_second = None;
        snapshot.eta_seconds = None;
        return;
    }
    let speed = (transferred as f64 / elapsed).round() as u64;
    snapshot.bytes_per_second = Some(speed);
    snapshot.eta_seconds = (speed > 0 && total > downloaded)
        .then(|| ((total - downloaded) as f64 / speed as f64).ceil() as u64);
}

fn hash_file(path: &PathBuf) -> YiyuResult<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

// Keep the existing check timestamp format explicit for the UI and future daily gating.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn automatic_check_due_at(path: &Path, now: u64) -> bool {
    let last_check = std::fs::read_to_string(path)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok());
    last_check.is_none_or(|last_check| now.saturating_sub(last_check) >= AUTO_CHECK_INTERVAL_SECS)
}

pub fn app_info(paths: &yiyu_core::paths::AppPaths) -> AppInfoDTO {
    AppInfoDTO {
        name: "Yiyu".into(),
        display_name: "译羽".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        identifier: yiyu_core::paths::APP_IDENTIFIER.into(),
        platform: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        data_dir: paths.root().display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use yiyu_core::settings::ProxyMode;

    fn release_info(
        installer_name: &str,
        installer_url: String,
        checksums_url: String,
        installer_size: u64,
    ) -> ReleaseInfo {
        ReleaseInfo {
            tag_name: "v0.2.0".into(),
            version: "0.2.0".into(),
            name: "Yiyu 0.2.0".into(),
            body: "Notes".into(),
            published_at: "2026-11-17T00:00:00Z".into(),
            html_url: "https://example.com/release".into(),
            installer: yiyu_core::update::ReleaseAsset {
                name: installer_name.into(),
                browser_download_url: installer_url,
                size: installer_size,
            },
            checksums: yiyu_core::update::ReleaseAsset {
                name: "SHA256SUMS.txt".into(),
                browser_download_url: checksums_url,
                size: 100,
            },
        }
    }

    fn sha256(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    #[test]
    fn rejects_unsafe_asset_names() {
        assert!(safe_asset_name("../evil.dmg").is_err());
        assert!(safe_asset_name("Yiyu_0.2.0_aarch64.dmg").is_ok());
    }

    #[test]
    fn progress_is_unknown_without_total() {
        assert_eq!(progress(10, 0), None);
        assert_eq!(progress(50, 100), Some(50.0));
    }

    #[test]
    fn update_and_provider_proxy_modes_are_independent() {
        let provider = ProxyConfig {
            mode: ProxyMode::Direct,
            url: None,
        };
        let update = ProxyConfig {
            mode: ProxyMode::Manual,
            url: Some("http://127.0.0.1:7890".into()),
        };
        assert_eq!(provider.mode, ProxyMode::Direct);
        assert_eq!(update.mode, ProxyMode::Manual);
    }

    #[test]
    fn automatic_update_check_is_limited_to_a_daily_window() {
        let dir = std::env::temp_dir().join(format!("yiyu-update-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let stamp = dir.join(LAST_CHECK_FILE);
        assert!(automatic_check_due_at(&stamp, 1_000));
        std::fs::write(&stamp, "1000").unwrap();
        assert!(!automatic_check_due_at(
            &stamp,
            AUTO_CHECK_INTERVAL_SECS - 1
        ));
        assert!(automatic_check_due_at(
            &stamp,
            1_000 + AUTO_CHECK_INTERVAL_SECS
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn transfer_metrics_report_speed_and_remaining_time() {
        let started = Instant::now();
        std::thread::sleep(Duration::from_millis(20));
        let mut snapshot = UpdateSnapshotDTO::idle("0.1.0".into());
        apply_transfer_metrics(&mut snapshot, 512, 1024, 0, started);
        assert!(snapshot.bytes_per_second.is_some_and(|speed| speed > 0));
        assert!(snapshot.eta_seconds.is_some_and(|eta| eta > 0));
    }

    #[tokio::test]
    async fn download_verifies_checksum_and_locks_the_installer_path() {
        let server = MockServer::start().await;
        let installer = b"verified installer bytes";
        let digest = sha256(installer);
        Mock::given(method("GET"))
            .and(path("/installer.dmg"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(installer))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/SHA256SUMS.txt"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(format!("{digest}  Yiyu.dmg\n")),
            )
            .mount(&server)
            .await;

        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let manager = UpdateManager::new(paths.clone());
        *manager.resolved.lock().unwrap() = Some(ResolvedUpdate {
            release: release_info(
                "Yiyu.dmg",
                format!("{}/installer.dmg", server.uri()),
                format!("{}/SHA256SUMS.txt", server.uri()),
                installer.len() as u64,
            ),
        });
        let ready = manager
            .download(&ProxyConfig::default(), |_| {})
            .await
            .unwrap();
        assert_eq!(ready.status, "ready");
        let ready_path = ready.file_path.unwrap();
        assert!(manager.verify_ready_file(&ready_path).is_ok());
        assert!(manager.verify_ready_file("/tmp/outside-installer").is_err());

        tokio::fs::write(&ready_path, b"tampered").await.unwrap();
        assert!(manager.verify_ready_file(&ready_path).is_err());
    }

    #[tokio::test]
    async fn checksum_mismatch_removes_the_partial_installer() {
        let server = MockServer::start().await;
        let installer = b"tampered installer bytes";
        Mock::given(method("GET"))
            .and(path("/installer.dmg"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(installer))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/SHA256SUMS.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("{}  Yiyu.dmg\n", "0".repeat(64))),
            )
            .mount(&server)
            .await;

        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let manager = UpdateManager::new(paths.clone());
        *manager.resolved.lock().unwrap() = Some(ResolvedUpdate {
            release: release_info(
                "Yiyu.dmg",
                format!("{}/installer.dmg", server.uri()),
                format!("{}/SHA256SUMS.txt", server.uri()),
                installer.len() as u64,
            ),
        });
        assert!(
            manager
                .download(&ProxyConfig::default(), |_| {})
                .await
                .is_err()
        );
        assert_eq!(manager.snapshot().status, "error");
        let final_path = paths.updates_dir().join("Yiyu.dmg");
        assert!(!final_path.exists());
        assert!(!part_file_path(&final_path).exists());
    }

    #[tokio::test]
    async fn resumed_download_uses_range_and_keeps_existing_bytes() {
        let server = MockServer::start().await;
        let installer = b"hello installer";
        let digest = sha256(installer);
        Mock::given(method("GET"))
            .and(path("/installer.dmg"))
            .and(header("range", "bytes=5-"))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(b" installer"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/SHA256SUMS.txt"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(format!("{digest}  Yiyu.dmg\n")),
            )
            .mount(&server)
            .await;

        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let final_path = paths.updates_dir().join("Yiyu.dmg");
        tokio::fs::write(part_file_path(&final_path), b"hello")
            .await
            .unwrap();
        let manager = UpdateManager::new(paths);
        *manager.resolved.lock().unwrap() = Some(ResolvedUpdate {
            release: release_info(
                "Yiyu.dmg",
                format!("{}/installer.dmg", server.uri()),
                format!("{}/SHA256SUMS.txt", server.uri()),
                installer.len() as u64,
            ),
        });
        let ready = manager
            .download(&ProxyConfig::default(), |_| {})
            .await
            .unwrap();
        assert_eq!(ready.status, "ready");
        assert_eq!(ready.downloaded_bytes, installer.len() as u64);
    }

    #[tokio::test]
    async fn completed_partial_is_verified_without_downloading_again() {
        let server = MockServer::start().await;
        let installer = b"already downloaded installer";
        Mock::given(method("GET"))
            .and(path("/installer.dmg"))
            .respond_with(ResponseTemplate::new(416))
            .expect(0)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/SHA256SUMS.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("{}  Yiyu.dmg\n", sha256(installer))),
            )
            .mount(&server)
            .await;
        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let part = part_file_path(&paths.updates_dir().join("Yiyu.dmg"));
        tokio::fs::write(&part, installer).await.unwrap();
        let manager = UpdateManager::new(paths);
        *manager.resolved.lock().unwrap() = Some(ResolvedUpdate {
            release: release_info(
                "Yiyu.dmg",
                format!("{}/installer.dmg", server.uri()),
                format!("{}/SHA256SUMS.txt", server.uri()),
                installer.len() as u64,
            ),
        });
        let ready = manager
            .download(&ProxyConfig::default(), |_| {})
            .await
            .unwrap();
        assert_eq!(ready.status, "ready");
        assert_eq!(
            tokio::fs::read(ready.file_path.unwrap()).await.unwrap(),
            installer
        );
    }

    #[tokio::test]
    async fn rejected_range_restarts_download_from_the_beginning() {
        let server = MockServer::start().await;
        let installer = b"hello installer";
        Mock::given(method("GET"))
            .and(path("/installer.dmg"))
            .and(header("range", "bytes=5-"))
            .respond_with(ResponseTemplate::new(416))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/installer.dmg"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(installer))
            .with_priority(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/SHA256SUMS.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("{}  Yiyu.dmg\n", sha256(installer))),
            )
            .mount(&server)
            .await;
        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        tokio::fs::write(
            part_file_path(&paths.updates_dir().join("Yiyu.dmg")),
            b"hello",
        )
        .await
        .unwrap();
        let manager = UpdateManager::new(paths);
        *manager.resolved.lock().unwrap() = Some(ResolvedUpdate {
            release: release_info(
                "Yiyu.dmg",
                format!("{}/installer.dmg", server.uri()),
                format!("{}/SHA256SUMS.txt", server.uri()),
                installer.len() as u64,
            ),
        });
        let ready = manager
            .download(&ProxyConfig::default(), |_| {})
            .await
            .unwrap();
        assert_eq!(ready.status, "ready");
        assert_eq!(
            tokio::fs::read(ready.file_path.unwrap()).await.unwrap(),
            installer
        );
    }

    #[tokio::test]
    async fn oversized_partial_is_replaced_by_a_fresh_download() {
        let server = MockServer::start().await;
        let installer = b"new installer";
        let oversized = b"an old partial file that is larger than the new installer";
        Mock::given(method("GET"))
            .and(path("/installer.dmg"))
            .and(header("range", format!("bytes={}-", oversized.len())))
            .respond_with(ResponseTemplate::new(416))
            .expect(0)
            .with_priority(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/installer.dmg"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(installer))
            .with_priority(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/SHA256SUMS.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("{}  Yiyu.dmg\n", sha256(installer))),
            )
            .mount(&server)
            .await;
        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        tokio::fs::write(
            part_file_path(&paths.updates_dir().join("Yiyu.dmg")),
            oversized,
        )
        .await
        .unwrap();
        let manager = UpdateManager::new(paths);
        *manager.resolved.lock().unwrap() = Some(ResolvedUpdate {
            release: release_info(
                "Yiyu.dmg",
                format!("{}/installer.dmg", server.uri()),
                format!("{}/SHA256SUMS.txt", server.uri()),
                installer.len() as u64,
            ),
        });
        let ready = manager
            .download(&ProxyConfig::default(), |_| {})
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read(ready.file_path.unwrap()).await.unwrap(),
            installer
        );
    }

    #[tokio::test]
    async fn cancellation_leaves_a_retryable_partial_file() {
        let server = MockServer::start().await;
        let installer = b"slow installer bytes";
        Mock::given(method("GET"))
            .and(path("/installer.dmg"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(250))
                    .set_body_bytes(installer),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/SHA256SUMS.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("{}  Yiyu.dmg\n", sha256(installer))),
            )
            .mount(&server)
            .await;

        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let part_path = part_file_path(&paths.updates_dir().join("Yiyu.dmg"));
        tokio::fs::write(&part_path, b"existing partial")
            .await
            .unwrap();
        let manager = std::sync::Arc::new(UpdateManager::new(paths.clone()));
        *manager.resolved.lock().unwrap() = Some(ResolvedUpdate {
            release: release_info(
                "Yiyu.dmg",
                format!("{}/installer.dmg", server.uri()),
                format!("{}/SHA256SUMS.txt", server.uri()),
                installer.len() as u64,
            ),
        });
        let task_manager = manager.clone();
        let task =
            tokio::spawn(
                async move { task_manager.download(&ProxyConfig::default(), |_| {}).await },
            );
        tokio::time::sleep(Duration::from_millis(40)).await;
        manager.cancel();
        let cancelled = tokio::time::timeout(Duration::from_millis(150), task)
            .await
            .expect("cancellation must interrupt a stalled network request")
            .unwrap()
            .unwrap();
        assert_eq!(cancelled.status, "cancelled");
        assert_eq!(
            tokio::fs::read(part_path).await.unwrap(),
            b"existing partial"
        );
    }

    #[tokio::test]
    async fn malformed_release_does_not_suppress_later_automatic_checks() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/release"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;
        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let manager = UpdateManager::new(paths);
        let result = manager
            .fetch_release_from_url(
                url::Url::parse(&format!("{}/release", server.uri())).unwrap(),
                &ProxyConfig::default(),
                "0.1.0",
            )
            .await;
        assert!(result.is_err());
        assert!(manager.automatic_check_due());
    }

    #[tokio::test]
    async fn repository_without_a_release_reports_no_available_update() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/release"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let manager = UpdateManager::new(paths);
        let snapshot = manager
            .fetch_release_from_url(
                url::Url::parse(&format!("{}/release", server.uri())).unwrap(),
                &ProxyConfig::default(),
                "0.1.0",
            )
            .await
            .unwrap();
        assert_eq!(snapshot.status, "noRelease");
    }

    #[tokio::test]
    async fn failed_recheck_discards_stale_release() {
        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let manager = UpdateManager::new(paths);
        *manager.resolved.lock().unwrap() = Some(ResolvedUpdate {
            release: release_info(
                "Yiyu.dmg",
                "https://example.com/old".into(),
                "https://example.com/sums".into(),
                10,
            ),
        });
        let proxy = ProxyConfig {
            mode: ProxyMode::Manual,
            url: None,
        };
        assert!(manager.check(&proxy, true).await.is_err());
        assert!(manager.resolved.lock().unwrap().is_none());
        assert_eq!(manager.snapshot().status, "error");
    }

    #[tokio::test]
    async fn update_operations_do_not_run_concurrently() {
        let temp = tempfile::tempdir().unwrap();
        let paths = yiyu_core::paths::AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let manager = UpdateManager::new(paths);
        let _guard = manager.operation.lock().await;
        let error = manager
            .download(&ProxyConfig::default(), |_| {})
            .await
            .unwrap_err();
        assert!(error.message.contains("another update operation"));
    }
}
