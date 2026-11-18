use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub label: String,
    pub base_url: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub extra_headers: BTreeMap<String, String>,
    pub is_custom: bool,
    pub requires_api_key: bool,
}

impl ProviderConfig {
    pub fn new(id: &str, label: &str, base_url: &str, model: &str, requires_api_key: bool) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            base_url: base_url.to_string(),
            model: model.to_string(),
            system_prompt: None,
            extra_headers: BTreeMap::new(),
            is_custom: false,
            requires_api_key,
        }
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.extra_headers
            .insert(name.to_string(), value.to_string());
        self
    }

    pub fn custom(label: String, base_url: String, model: String) -> Self {
        Self {
            id: format!("custom-{}", uuid::Uuid::new_v4()),
            label,
            base_url,
            model,
            system_prompt: None,
            extra_headers: BTreeMap::new(),
            is_custom: true,
            requires_api_key: true,
        }
    }

    pub fn chat_completions_url(&self) -> crate::YiyuResult<url::Url> {
        self.endpoint_url(&["chat", "completions"])
    }

    pub fn models_url(&self) -> crate::YiyuResult<url::Url> {
        self.endpoint_url(&["models"])
    }

    fn endpoint_url(&self, parts: &[&str]) -> crate::YiyuResult<url::Url> {
        let mut url = url::Url::parse(self.base_url.trim())?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(crate::YiyuError::validation(
                "base URL must be http(s)",
                "baseUrl",
            ));
        }
        {
            let mut segments = url.path_segments_mut().map_err(|_| {
                crate::YiyuError::validation("base URL cannot contain endpoint paths", "baseUrl")
            })?;
            segments.pop_if_empty();
            for part in parts {
                segments.push(part);
            }
        }
        Ok(url)
    }
}

pub fn built_in_providers() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig::new(
            "openai",
            "OpenAI",
            "https://api.openai.com/v1",
            "gpt-6-luna",
            true,
        ),
        ProviderConfig::new(
            "deepseek",
            "DeepSeek",
            "https://api.deepseek.com/v1",
            "deepseek-flash",
            true,
        ),
        ProviderConfig::new(
            "openrouter",
            "OpenRouter",
            "https://openrouter.ai/api/v1",
            "openrouter/auto",
            true,
        )
        .with_header("HTTP-Referer", "https://github.com/Goldloli/yiyu")
        .with_header("X-Title", APP_NAME),
        ProviderConfig::new(
            "zhipu",
            "智谱 GLM",
            "https://open.bigmodel.cn/api/paas/v4",
            "glm-5.3-flash",
            true,
        ),
        ProviderConfig::new(
            "kimi",
            "Kimi",
            "https://api.moonshot.cn/v1",
            "kimi-k3.1-pro",
            true,
        ),
        ProviderConfig::new(
            "siliconflow",
            "硅基流动",
            "https://api.siliconflow.cn/v1",
            "deepseek-ai/DeepSeek-V4.1-Flash",
            true,
        ),
        ProviderConfig::new(
            "ollama",
            "Ollama",
            "http://127.0.0.1:11434/v1",
            "qwen3.5:4b",
            false,
        ),
        ProviderConfig::new(
            "lmstudio",
            "LM Studio",
            "http://127.0.0.1:1234/v1",
            "local-model",
            false,
        ),
    ]
}

const APP_NAME: &str = "Yiyu";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_round_trip_and_urls_are_stable() {
        let provider = ProviderConfig::new(
            "demo",
            "Demo",
            "https://example.com/v1/",
            "demo-model",
            true,
        );
        let encoded = serde_json::to_string(&provider).unwrap();
        let decoded: ProviderConfig = serde_json::from_str(&encoded).unwrap();
        assert_eq!(provider, decoded);
        assert_eq!(
            decoded.chat_completions_url().unwrap().as_str(),
            "https://example.com/v1/chat/completions"
        );
    }

    #[test]
    fn endpoint_paths_are_inserted_before_base_url_query() {
        let provider = ProviderConfig::new(
            "test",
            "Test",
            "https://example.com/v1/?tenant=sample",
            "model",
            false,
        );
        assert_eq!(
            provider.models_url().unwrap().as_str(),
            "https://example.com/v1/models?tenant=sample"
        );
        assert_eq!(
            provider.chat_completions_url().unwrap().as_str(),
            "https://example.com/v1/chat/completions?tenant=sample"
        );
    }

    #[test]
    fn custom_provider_is_marked_custom() {
        let provider = ProviderConfig::custom(
            "Local".into(),
            "http://127.0.0.1:8000/v1".into(),
            "local".into(),
        );
        assert!(provider.is_custom);
        assert!(provider.id.starts_with("custom-"));
    }

    #[test]
    fn builtins_match_the_release_catalog() {
        let providers = built_in_providers();
        assert_eq!(providers.len(), 8);
        assert!(providers.iter().any(|provider| provider.id == "ollama"));
        assert!(providers.iter().any(|provider| provider.id == "openrouter"));
    }
}
