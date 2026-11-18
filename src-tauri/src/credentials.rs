use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

const SERVICE: &str = "com.yiyu.desktop.provider";

#[derive(Clone, Default)]
pub struct CredentialStore;

impl CredentialStore {
    pub fn set(&self, provider_id: &str, value: &str) -> YiyuResult<()> {
        let entry = keyring::Entry::new(SERVICE, provider_id).map_err(credential_error)?;
        entry.set_password(value).map_err(credential_error)
    }

    pub fn get(&self, provider_id: &str) -> YiyuResult<Option<String>> {
        let entry = keyring::Entry::new(SERVICE, provider_id).map_err(credential_error)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(credential_error(error)),
        }
    }

    pub fn delete(&self, provider_id: &str) -> YiyuResult<()> {
        let entry = keyring::Entry::new(SERVICE, provider_id).map_err(credential_error)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(credential_error(error)),
        }
    }
}

fn credential_error(error: keyring::Error) -> YiyuError {
    YiyuError::new(
        ErrorKind::Storage,
        format!("system credential store error: {error}"),
    )
    .field("apiKey")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_service_is_namespaced() {
        assert!(SERVICE.starts_with("com.yiyu.desktop"));
    }

    #[test]
    fn settings_schema_has_no_raw_api_key_field() {
        let settings = yiyu_core::settings::Settings {
            active_provider_id: Some("openai".into()),
            ..Default::default()
        };
        let serialized = serde_json::to_string(&settings).unwrap();
        assert!(!serialized.contains("apiKey"));
        assert!(!serialized.contains("sk-"));
        assert!(!serialized.contains("Authorization"));
    }
}
