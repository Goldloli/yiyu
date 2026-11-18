use crate::{ErrorKind, YiyuError, YiyuResult};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub version: String,
    pub name: String,
    pub body: String,
    pub published_at: String,
    pub html_url: String,
    pub installer: ReleaseAsset,
    pub checksums: ReleaseAsset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdatePlatform {
    MacosAarch64,
    WindowsX64,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

pub fn parse_github_release(body: &str, platform: UpdatePlatform) -> YiyuResult<ReleaseInfo> {
    let release: GitHubRelease = serde_json::from_str(body)?;
    let version_string = release.tag_name.trim_start_matches('v');
    let version = Version::parse(version_string).map_err(|error| {
        YiyuError::new(
            ErrorKind::Update,
            format!("invalid release version: {error}"),
        )
    })?;

    let installer = match platform {
        UpdatePlatform::MacosAarch64 => select_unique(
            &release.assets,
            |name| name.ends_with("_aarch64.dmg"),
            "macOS aarch64 DMG",
        )?,
        UpdatePlatform::WindowsX64 => select_unique(
            &release.assets,
            |name| name.ends_with("_x64-setup.exe") || name.ends_with("_x64.exe"),
            "Windows x64 NSIS installer",
        )?,
    };
    let checksums = select_unique(
        &release.assets,
        |name| {
            name.eq_ignore_ascii_case("SHA256SUMS.txt") || name.eq_ignore_ascii_case("SHA256SUMS")
        },
        "SHA256SUMS",
    )?;

    Ok(ReleaseInfo {
        tag_name: release.tag_name,
        version: version.to_string(),
        name: release.name.unwrap_or_else(|| version.to_string()),
        body: release.body.unwrap_or_default(),
        published_at: release.published_at.unwrap_or_default(),
        html_url: release.html_url,
        installer,
        checksums,
    })
}

fn select_unique(
    assets: &[GitHubAsset],
    predicate: impl Fn(&str) -> bool,
    label: &str,
) -> YiyuResult<ReleaseAsset> {
    let matches: Vec<&GitHubAsset> = assets
        .iter()
        .filter(|asset| predicate(&asset.name))
        .collect();
    if matches.len() != 1 {
        return Err(YiyuError::new(
            ErrorKind::Update,
            format!(
                "expected exactly one {label} asset, found {}",
                matches.len()
            ),
        ));
    }
    let asset = matches[0];
    Ok(ReleaseAsset {
        name: asset.name.clone(),
        browser_download_url: asset.browser_download_url.clone(),
        size: asset.size,
    })
}

pub fn parse_sha256_sums(body: &str) -> HashMap<String, String> {
    body.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?.trim();
            let name = parts
                .next()?
                .trim_start_matches('*')
                .trim_start_matches("./");
            if hash.len() != 64 || !hash.chars().all(|character| character.is_ascii_hexdigit()) {
                return None;
            }
            Some((name.to_string(), hash.to_ascii_lowercase()))
        })
        .collect()
}

pub fn expected_sha256<'a>(sums: &'a HashMap<String, String>, name: &str) -> YiyuResult<&'a str> {
    sums.get(name)
        .map(String::as_str)
        .ok_or_else(|| YiyuError::new(ErrorKind::Update, format!("checksum missing for {name}")))
}

pub fn is_newer(current: &str, candidate: &str) -> YiyuResult<bool> {
    let current = Version::parse(current).map_err(|error| {
        YiyuError::new(
            ErrorKind::Update,
            format!("invalid current version: {error}"),
        )
    })?;
    let candidate = Version::parse(candidate).map_err(|error| {
        YiyuError::new(
            ErrorKind::Update,
            format!("invalid candidate version: {error}"),
        )
    })?;
    Ok(candidate > current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_body() -> String {
        serde_json::json!({
            "tag_name": "v0.2.0",
            "name": "Yiyu 0.2.0",
            "body": "Notes",
            "published_at": "2026-11-17T00:00:00Z",
            "html_url": "https://github.com/example/yiyu/releases/tag/v0.2.0",
            "assets": [
                {"name":"Yiyu_0.2.0_aarch64.dmg","browser_download_url":"https://example.com/mac.dmg","size":5000},
                {"name":"Yiyu_0.2.0_x64-setup.exe","browser_download_url":"https://example.com/win.exe","size":5001},
                {"name":"SHA256SUMS.txt","browser_download_url":"https://example.com/sums","size":100}
            ]
        })
        .to_string()
    }

    #[test]
    fn selects_exact_platform_asset() {
        let mac = parse_github_release(&release_body(), UpdatePlatform::MacosAarch64).unwrap();
        assert!(mac.installer.name.ends_with("aarch64.dmg"));
        let win = parse_github_release(&release_body(), UpdatePlatform::WindowsX64).unwrap();
        assert!(win.installer.name.ends_with("x64-setup.exe"));
    }

    #[test]
    fn ambiguous_assets_are_rejected() {
        let mut body: serde_json::Value = serde_json::from_str(&release_body()).unwrap();
        body["assets"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name":"Yiyu_0.2.0_other_aarch64.dmg",
                "browser_download_url":"https://example.com/other.dmg",
                "size":1
            }));
        assert!(parse_github_release(&body.to_string(), UpdatePlatform::MacosAarch64).is_err());
    }

    #[test]
    fn checksum_parser_and_version_comparison_work() {
        let sums = parse_sha256_sums(
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  Yiyu.dmg\n",
        );
        assert_eq!(sums.len(), 1);
        assert!(expected_sha256(&sums, "Yiyu.dmg").is_ok());
        assert!(is_newer("0.1.0", "0.2.0").unwrap());
        assert!(!is_newer("0.2.0", "0.2.0").unwrap());
    }

    #[test]
    fn checksum_parser_accepts_release_workflow_relative_paths() {
        let sums = parse_sha256_sums(
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  ./Yiyu.dmg\n",
        );
        assert!(expected_sha256(&sums, "Yiyu.dmg").is_ok());
    }
}
