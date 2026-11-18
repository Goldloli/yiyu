use std::path::Path;
use tracing_subscriber::filter::EnvFilter;

pub fn init(log_dir: &Path) -> YiyuResult<()> {
    std::fs::create_dir_all(log_dir)?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "yiyu.log");
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("yiyu_app=info,yiyu_core=info,warn"));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_appender)
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).map_err(|error| {
        YiyuError::new(
            ErrorKind::Other,
            format!("cannot initialize logging: {error}"),
        )
    })?;
    Ok(())
}

pub fn record_request_failure(
    provider_id: &str,
    model: &str,
    status: Option<u16>,
    duration_ms: u64,
    error: &YiyuError,
) {
    tracing::warn!(
        target: "yiyu_app::translation",
        provider = provider_id,
        model,
        status = ?status,
        duration_ms,
        error_kind = ?error.kind,
        retryable = error.retryable,
        "translation request failed"
    );
}

use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_error_contents_are_not_written_to_logs() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct Buffer(Arc<Mutex<Vec<u8>>>);
        impl Write for Buffer {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let output = bytes.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || Buffer(output.clone()))
            .finish();
        let error = YiyuError::new(ErrorKind::RateLimit, "secret-key and private source text");
        tracing::subscriber::with_default(subscriber, || {
            record_request_failure("provider", "model", Some(429), 25, &error);
        });
        let log = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
        assert!(log.contains("RateLimit"));
        assert!(!log.contains("secret-key"));
        assert!(!log.contains("private source text"));
    }

    #[test]
    fn diagnostic_line_has_no_source_text_slot() {
        let line = format!(
            "provider={} model={} status={:?} duration_ms={}",
            "openai",
            "gpt",
            Some(429),
            42
        );
        assert!(!line.contains("sourceText"));
    }
}
