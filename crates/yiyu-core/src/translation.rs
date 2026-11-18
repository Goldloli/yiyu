use crate::language::{LanguageDirection, resolve_direction};
use crate::prompt::resolve_system_prompt;
use crate::provider::ProviderConfig;
use crate::settings::ProxyConfig;
use crate::sse::{OpenAiEvent, SseDecoder, extract_error_message, extract_full_content};
use crate::transport::build_client;
use crate::{ErrorKind, YiyuError, YiyuResult};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationRequest {
    pub provider: ProviderConfig,
    pub api_key: Option<String>,
    pub text: String,
    pub manual_source_language: Option<String>,
    pub target_language: String,
    pub fallback_target_language: String,
    pub proxy: ProxyConfig,
    pub max_input_chars: u32,
    pub ui_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationOutcome {
    pub text: String,
    pub direction: LanguageDirection,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTestResult {
    pub ok: bool,
    pub latency_ms: u64,
    pub message: String,
}

pub async fn translate_stream<F>(
    request: &TranslationRequest,
    cancellation: &CancellationToken,
    mut on_delta: F,
) -> YiyuResult<TranslationOutcome>
where
    F: FnMut(&str),
{
    validate_request(request)?;
    let direction = resolve_direction(
        &request.text,
        request.manual_source_language.as_deref(),
        &request.target_language,
        &request.fallback_target_language,
        &request.ui_language,
    );
    let body = build_chat_body(request, &direction);
    let url = request.provider.chat_completions_url()?;
    let client = build_client(&request.proxy, &url)?;
    let http_request = configure_provider_request(
        client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&body),
        &request.provider,
        request.api_key.as_deref(),
    );

    let started = Instant::now();
    let response = tokio::select! {
        _ = cancellation.cancelled() => return Err(YiyuError::cancelled()),
        response = http_request.send() => response?,
    };
    let status = response.status();
    if !status.is_success() {
        let body = tokio::select! {
            _ = cancellation.cancelled() => return Err(YiyuError::cancelled()),
            body = response.text() => body.unwrap_or_default(),
        };
        return Err(map_status_error(
            status.as_u16(),
            extract_error_message(&body),
        ));
    }

    let is_sse = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("text/event-stream"));
    let mut output = String::new();

    if is_sse {
        let mut decoder = SseDecoder::default();
        let mut stream = response.bytes_stream();
        loop {
            let chunk = tokio::select! {
                _ = cancellation.cancelled() => return Err(YiyuError::cancelled()),
                chunk = stream.next() => chunk,
            };
            let Some(chunk) = chunk else {
                break;
            };
            let bytes = chunk?;
            let events = decoder.push_chunk(&bytes)?;
            for event in events {
                match event {
                    OpenAiEvent::Delta(delta) => {
                        output.push_str(&delta);
                        on_delta(&delta);
                    }
                    OpenAiEvent::Done => {
                        return finish_output(
                            output,
                            direction,
                            started.elapsed().as_millis() as u64,
                        );
                    }
                }
            }
        }
        for event in decoder.finish()? {
            if let OpenAiEvent::Delta(delta) = event {
                output.push_str(&delta);
                on_delta(&delta);
            }
        }
    } else {
        let body = tokio::select! {
            _ = cancellation.cancelled() => return Err(YiyuError::cancelled()),
            body = response.text() => body?,
        };
        output = extract_full_content(&body)?;
        on_delta(&output);
    }

    finish_output(output, direction, started.elapsed().as_millis() as u64)
}

pub async fn test_provider(
    provider: &ProviderConfig,
    api_key: Option<&str>,
    proxy: &ProxyConfig,
) -> ProviderTestResult {
    let started = Instant::now();
    let result =
        test_provider_inner(provider, api_key, proxy, std::time::Duration::from_secs(15)).await;
    let latency_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(message) => ProviderTestResult {
            ok: true,
            latency_ms,
            message,
        },
        Err(error) => ProviderTestResult {
            ok: false,
            latency_ms,
            message: error.message,
        },
    }
}

async fn test_provider_inner(
    provider: &ProviderConfig,
    api_key: Option<&str>,
    proxy: &ProxyConfig,
    timeout: std::time::Duration,
) -> YiyuResult<String> {
    let url = provider.models_url()?;
    let client = build_client(proxy, &url)?;
    let request = configure_provider_request(client.get(url), provider, api_key);
    let response = request.timeout(timeout).send().await?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(map_status_error(status, extract_error_message(&body)));
    }
    Ok("connection succeeded".into())
}

pub async fn fetch_models(
    provider: &ProviderConfig,
    api_key: Option<&str>,
    proxy: &ProxyConfig,
) -> YiyuResult<Vec<String>> {
    let url = provider.models_url()?;
    let client = build_client(proxy, &url)?;
    let request = configure_provider_request(client.get(url), provider, api_key);
    let response = request
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(map_status_error(status, extract_error_message(&body)));
    }
    let value: serde_json::Value = response.json().await?;
    let mut models: Vec<String> = value
        .get("data")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| model.get("id").and_then(serde_json::Value::as_str))
        .filter(|id| !obvious_non_chat_model(id))
        .map(str::to_string)
        .collect();
    models.sort();
    models.dedup();
    Ok(models)
}

fn configure_provider_request(
    mut request: reqwest::RequestBuilder,
    provider: &ProviderConfig,
    api_key: Option<&str>,
) -> reqwest::RequestBuilder {
    if let Some(key) = api_key.map(str::trim).filter(|value| !value.is_empty()) {
        request = request.bearer_auth(key);
    }
    for (name, value) in &provider.extra_headers {
        request = request.header(name, value);
    }
    request
}

fn build_chat_body(
    request: &TranslationRequest,
    direction: &LanguageDirection,
) -> serde_json::Value {
    let prompt = resolve_system_prompt(
        request.provider.system_prompt.as_deref(),
        &direction.source_label,
        &direction.target_label,
    );
    let mut body = json!({
        "model": request.provider.model,
        "stream": true,
        "messages": [
            {"role": "system", "content": prompt},
            {"role": "user", "content": request.text.trim_matches(|c: char| c.is_whitespace())}
        ]
    });
    // New OpenAI reasoning models reject temperature with their default effort.
    // Auto-routing can also select one of those models, so let it choose defaults.
    let model = request
        .provider
        .model
        .strip_prefix("openai/")
        .unwrap_or(&request.provider.model);
    if !model.starts_with("gpt-5") && !model.starts_with("gpt-6") && model != "openrouter/auto" {
        body["temperature"] = json!(0.2);
    }
    body
}

fn validate_request(request: &TranslationRequest) -> YiyuResult<()> {
    let text = request.text.trim();
    if text.is_empty() {
        return Err(YiyuError::validation("empty text", "text"));
    }
    if text.chars().count() > request.max_input_chars as usize {
        return Err(YiyuError::validation(
            format!(
                "input exceeds the current limit of {} characters",
                request.max_input_chars
            ),
            "maxInputChars",
        ));
    }
    if request.provider.base_url.trim().is_empty() {
        return Err(YiyuError::validation("base URL is required", "baseUrl"));
    }
    if request.provider.model.trim().is_empty() {
        return Err(YiyuError::validation("model is required", "model"));
    }
    if request.provider.requires_api_key
        && request
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err(
            YiyuError::new(ErrorKind::Authentication, "API key not configured")
                .field("apiKey")
                .action("open-settings"),
        );
    }
    Ok(())
}

fn finish_output(
    output: String,
    direction: LanguageDirection,
    duration_ms: u64,
) -> YiyuResult<TranslationOutcome> {
    if output.trim().is_empty() {
        return Err(
            YiyuError::new(ErrorKind::EmptyResult, "translation returned empty result")
                .retryable(true),
        );
    }
    Ok(TranslationOutcome {
        text: output,
        direction,
        duration_ms,
    })
}

fn map_status_error(status: u16, message: String) -> YiyuError {
    match status {
        401 | 403 => YiyuError::new(ErrorKind::Authentication, message).action("open-settings"),
        429 => YiyuError::new(ErrorKind::RateLimit, message).retryable(true),
        _ => YiyuError::new(
            ErrorKind::Protocol,
            format!("provider returned HTTP {status}: {message}"),
        )
        .retryable(status >= 500),
    }
}

fn obvious_non_chat_model(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    [
        "embedding",
        "embed",
        "whisper",
        "moderation",
        "tts",
        "dall-e",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderConfig;
    use std::time::Duration;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn request(text: &str, key: Option<&str>) -> TranslationRequest {
        TranslationRequest {
            provider: ProviderConfig::new("test", "Test", "http://127.0.0.1:9/v1", "model", true),
            api_key: key.map(str::to_string),
            text: text.into(),
            manual_source_language: None,
            target_language: "zh-Hans".into(),
            fallback_target_language: "en".into(),
            proxy: ProxyConfig::default(),
            max_input_chars: 20_000,
            ui_language: "zh".into(),
        }
    }

    #[test]
    fn empty_text_is_rejected_before_network() {
        let error = validate_request(&request(" \n\t ", Some("key"))).unwrap_err();
        assert_eq!(error.field.as_deref(), Some("text"));
    }

    #[test]
    fn configured_key_is_required_when_provider_requires_it() {
        let error = validate_request(&request("hello", None)).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Authentication);
    }

    #[test]
    fn input_limit_is_enforced() {
        let mut request = request("hello", Some("key"));
        request.max_input_chars = 1_000;
        request.text = "a".repeat(1_001);
        assert!(validate_request(&request).is_err());
    }

    #[test]
    fn chat_request_uses_only_translation_prompt_and_user_text() {
        let request = request("hello", Some("key"));
        let direction = resolve_direction("hello", None, "zh-Hans", "en", "zh");
        let body = build_chat_body(&request, &direction);
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"][1]["content"], "hello");
    }

    #[test]
    fn reasoning_models_and_auto_routing_omit_unsupported_temperature() {
        let direction = resolve_direction("hello", None, "zh-Hans", "en", "zh");
        let mut req = request("hello", Some("key"));
        for model in [
            "gpt-6-luna",
            "gpt-6-astra",
            "openai/gpt-6.1-sol",
            "openrouter/auto",
        ] {
            req.provider.model = model.into();
            assert!(
                build_chat_body(&req, &direction)
                    .get("temperature")
                    .is_none()
            );
        }
        req.provider.model = "gpt-4o-mini".into();
        assert_eq!(build_chat_body(&req, &direction)["temperature"], 0.2);
    }

    #[tokio::test]
    async fn streams_sse_translation_and_authorizes_the_request() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer secret"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                concat!(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n\n",
                    "data: {\"choices\":[{\"delta\":{\"content\":\"好\"}}]}\n\n",
                    "data: [DONE]\n\n"
                ),
                "text/event-stream",
            ))
            .expect(1)
            .mount(&server)
            .await;

        let mut request = request("hello", Some("secret"));
        request.provider.base_url = format!("{}/v1", server.uri());
        let mut deltas = Vec::new();
        let outcome = translate_stream(&request, &CancellationToken::new(), |delta| {
            deltas.push(delta.to_string());
        })
        .await
        .unwrap();

        assert_eq!(deltas, ["你", "好"]);
        assert_eq!(outcome.text, "你好");
    }

    #[tokio::test]
    async fn stream_errors_are_not_returned_as_successful_translations() {
        for payload in [
            "data: {bad\n\n",
            "data: {\"error\":{\"message\":\"model unavailable\"}}\n\n",
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/v1/chat/completions"))
                .respond_with(ResponseTemplate::new(200).set_body_raw(payload, "text/event-stream"))
                .mount(&server)
                .await;

            let mut request = request("hello", Some("secret"));
            request.provider.base_url = format!("{}/v1", server.uri());
            let mut deltas = Vec::new();
            let error = translate_stream(&request, &CancellationToken::new(), |delta| {
                deltas.push(delta.to_owned());
            })
            .await
            .unwrap_err();
            assert_eq!(error.kind, ErrorKind::Protocol);
            assert!(deltas.is_empty());
        }
    }

    #[tokio::test]
    async fn non_sse_json_translation_is_supported() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!({
                        "choices": [{"message": {"content": "你好"}}]
                    })),
            )
            .mount(&server)
            .await;

        let mut request = request("hello", Some("secret"));
        request.provider.base_url = format!("{}/v1", server.uri());
        let outcome = translate_stream(&request, &CancellationToken::new(), |_| {})
            .await
            .unwrap();
        assert_eq!(outcome.text, "你好");
    }

    #[tokio::test]
    async fn authentication_and_rate_limit_errors_are_normalized() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer bad-key"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(json!({"error": {"message": "invalid key"}})),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer limited-key"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_json(json!({"error": {"message": "slow down"}})),
            )
            .mount(&server)
            .await;

        let mut auth_request = request("hello", Some("bad-key"));
        auth_request.provider.base_url = format!("{}/v1", server.uri());
        let auth_error = translate_stream(&auth_request, &CancellationToken::new(), |_| {})
            .await
            .unwrap_err();
        assert_eq!(auth_error.kind, ErrorKind::Authentication);
        assert!(auth_error.actions.contains(&"open-settings".into()));

        let mut rate_request = request("hello", Some("limited-key"));
        rate_request.provider.base_url = format!("{}/v1", server.uri());
        let rate_error = translate_stream(&rate_request, &CancellationToken::new(), |_| {})
            .await
            .unwrap_err();
        assert_eq!(rate_error.kind, ErrorKind::RateLimit);
        assert!(rate_error.retryable);
    }

    #[tokio::test]
    async fn empty_provider_results_are_retryable() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!({"choices": []})),
            )
            .mount(&server)
            .await;

        let mut request = request("hello", Some("secret"));
        request.provider.base_url = format!("{}/v1", server.uri());
        let error = translate_stream(&request, &CancellationToken::new(), |_| {})
            .await
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::EmptyResult);
        assert!(error.retryable);
    }

    #[tokio::test]
    async fn connection_tests_succeed_filter_models_and_time_out() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("authorization", "Bearer secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {"id": "gpt-4.1-mini"},
                    {"id": "text-embedding-3-small"},
                    {"id": "gpt-4.1-mini"}
                ]
            })))
            .mount(&server)
            .await;

        let provider = ProviderConfig::new(
            "test",
            "Test",
            &format!("{}/v1", server.uri()),
            "gpt-4.1-mini",
            true,
        );
        let test = test_provider(&provider, Some("secret"), &ProxyConfig::default()).await;
        assert!(test.ok);
        let models = fetch_models(&provider, Some("secret"), &ProxyConfig::default())
            .await
            .unwrap();
        assert_eq!(models, ["gpt-4.1-mini"]);

        let slow = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(200))
                    .set_body_json(json!({"data": []})),
            )
            .mount(&slow)
            .await;
        let slow_provider = ProviderConfig::new(
            "slow",
            "Slow",
            &format!("{}/v1", slow.uri()),
            "model",
            false,
        );
        let error = test_provider_inner(
            &slow_provider,
            None,
            &ProxyConfig::default(),
            Duration::from_millis(20),
        )
        .await
        .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Timeout);
    }

    #[tokio::test]
    async fn provider_headers_are_used_for_connection_and_model_list() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("x-tenant", "translation-team"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{"id": "example-model"}]
            })))
            .expect(2)
            .mount(&server)
            .await;
        let mut provider = ProviderConfig::new(
            "test",
            "Test",
            &format!("{}/v1", server.uri()),
            "example-model",
            false,
        );
        provider
            .extra_headers
            .insert("X-Tenant".into(), "translation-team".into());
        assert!(
            test_provider(&provider, None, &ProxyConfig::default())
                .await
                .ok
        );
        assert_eq!(
            fetch_models(&provider, None, &ProxyConfig::default())
                .await
                .unwrap(),
            ["example-model"]
        );
    }

    #[tokio::test]
    async fn cancellation_interrupts_a_stalled_error_response_body() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 8192];
            let _ = stream.read(&mut request);
            stream
                .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 100\r\n\r\n")
                .unwrap();
            std::thread::sleep(Duration::from_millis(600));
        });
        let mut request = request("hello", Some("secret"));
        request.provider.base_url = format!("http://{address}/v1");
        let token = CancellationToken::new();
        let cancel = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancel.cancel();
        });
        let result = tokio::time::timeout(
            Duration::from_millis(300),
            translate_stream(&request, &token, |_| {}),
        )
        .await;
        server.join().unwrap();
        assert_eq!(
            result
                .expect("cancellation must not wait for the response body")
                .unwrap_err()
                .kind,
            ErrorKind::Cancelled
        );
    }
}
