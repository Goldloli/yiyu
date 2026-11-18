use crate::settings::{ProxyConfig, ProxyMode};
use crate::{ErrorKind, YiyuError, YiyuResult};
use reqwest::{Client, Url};
use std::time::Duration;
use url::Host;

pub fn build_client(proxy: &ProxyConfig, target: &Url) -> YiyuResult<Client> {
    let mut builder = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .user_agent(format!("Yiyu/{}", env!("CARGO_PKG_VERSION")));

    let local = is_localhost(target);
    match (&proxy.mode, local) {
        (_, true) | (ProxyMode::Direct, false) => {
            builder = builder.no_proxy();
        }
        (ProxyMode::System, false) => {
            // reqwest's system-proxy feature supplies OS/environment proxy settings.
        }
        (ProxyMode::Manual, false) => {
            let Some(url) = proxy
                .url
                .as_deref()
                .map(str::trim)
                .filter(|url| !url.is_empty())
            else {
                return Err(YiyuError::validation(
                    "manual proxy URL is required",
                    "proxy.url",
                ));
            };
            let reqwest_proxy = reqwest::Proxy::all(url).map_err(|error| {
                YiyuError::validation(format!("invalid proxy URL: {error}"), "proxy.url")
            })?;
            builder = builder.proxy(reqwest_proxy);
        }
    }

    builder.build().map_err(|error| {
        YiyuError::new(
            ErrorKind::Network,
            format!("cannot initialize HTTP transport: {error}"),
        )
    })
}

pub fn is_localhost(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

pub fn valid_proxy_url(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    (lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("socks5://")
        || lower.starts_with("socks5h://"))
        && Url::parse(value).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localhost_is_detected() {
        for value in [
            "http://localhost:1234/v1",
            "http://127.0.0.1:1234/v1",
            "http://[::1]:1234/v1",
        ] {
            assert!(is_localhost(&Url::parse(value).unwrap()));
        }
        assert!(!is_localhost(
            &Url::parse("https://api.openai.com/v1").unwrap()
        ));
    }

    #[test]
    fn supported_proxy_schemes_are_accepted() {
        for value in [
            "http://127.0.0.1:7890",
            "https://proxy.example.com",
            "socks5://127.0.0.1:7891",
        ] {
            assert!(valid_proxy_url(value));
        }
        assert!(!valid_proxy_url("ftp://proxy"));
    }

    #[test]
    fn localhost_builds_even_with_system_proxy() {
        let url = Url::parse("http://127.0.0.1:11434/v1").unwrap();
        assert!(build_client(&ProxyConfig::default(), &url).is_ok());
    }
}
