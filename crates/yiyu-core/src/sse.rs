use crate::{ErrorKind, YiyuError, YiyuResult};
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
    data_lines: Vec<String>,
    seen_first_line: bool,
    scan_from: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAiEvent {
    Delta(String),
    Done,
}

impl SseDecoder {
    pub fn push_chunk(&mut self, chunk: &[u8]) -> YiyuResult<Vec<OpenAiEvent>> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        let mut lines = Vec::new();
        let mut start = 0;
        let mut index = self.scan_from;
        while index < self.buffer.len() {
            let next = match self.buffer[index] {
                b'\n' => index + 1,
                b'\r' if index + 1 == self.buffer.len() => break,
                b'\r' if self.buffer[index + 1] == b'\n' => index + 2,
                b'\r' => index + 1,
                _ => {
                    index += 1;
                    continue;
                }
            };
            lines.push(self.buffer[start..index].to_vec());
            start = next;
            index = next;
        }
        self.buffer.drain(..start);
        self.scan_from = index - start;
        for line in lines {
            self.consume_line(&line, &mut events)?;
            if matches!(events.last(), Some(OpenAiEvent::Done)) {
                self.buffer.clear();
                self.scan_from = 0;
                break;
            }
        }
        Ok(events)
    }

    pub fn finish(&mut self) -> YiyuResult<Vec<OpenAiEvent>> {
        let mut events = Vec::new();
        self.scan_from = 0;
        if !self.buffer.is_empty() {
            let mut line = std::mem::take(&mut self.buffer);
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            self.consume_line(&line, &mut events)?;
        }
        self.flush_block(&mut events)?;
        Ok(events)
    }

    fn consume_line(&mut self, line: &[u8], events: &mut Vec<OpenAiEvent>) -> YiyuResult<()> {
        let mut line = std::str::from_utf8(line).map_err(|_| {
            YiyuError::new(ErrorKind::Protocol, "stream contains invalid UTF-8").retryable(true)
        })?;
        if !self.seen_first_line {
            self.seen_first_line = true;
            line = line.strip_prefix('\u{feff}').unwrap_or(line);
        }
        if line.is_empty() {
            return self.flush_block(events);
        }
        if let Some(data) = line.strip_prefix("data:") {
            self.data_lines
                .push(data.strip_prefix(' ').unwrap_or(data).to_string());
        }
        Ok(())
    }

    fn flush_block(&mut self, events: &mut Vec<OpenAiEvent>) -> YiyuResult<()> {
        if self.data_lines.is_empty() {
            return Ok(());
        }
        let payload = self.data_lines.join("\n");
        self.data_lines.clear();
        if payload.trim() == "[DONE]" {
            events.push(OpenAiEvent::Done);
            return Ok(());
        }
        if let Some(delta) = extract_openai_delta(&payload)?
            && !delta.is_empty()
        {
            events.push(OpenAiEvent::Delta(delta));
        }
        Ok(())
    }
}

pub fn extract_openai_delta(payload: &str) -> YiyuResult<Option<String>> {
    let value: Value = serde_json::from_str(payload)?;
    if value.get("error").is_some_and(|error| !error.is_null()) {
        return Err(YiyuError::new(
            ErrorKind::Protocol,
            extract_error_message_value(&value).unwrap_or_else(|| "provider stream failed".into()),
        )
        .retryable(true));
    }
    Ok(value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("delta"))
        .and_then(|delta| delta.get("content"))
        .and_then(Value::as_str)
        .map(str::to_string))
}

pub fn extract_full_content(payload: &str) -> YiyuResult<String> {
    let value: Value = serde_json::from_str(payload)?;
    if let Some(error) = extract_error_message_value(&value) {
        return Err(YiyuError::new(ErrorKind::Protocol, error));
    }
    value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .or_else(|| {
            value
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first())
                .and_then(|choice| choice.get("text"))
        })
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            YiyuError::new(
                ErrorKind::EmptyResult,
                "response does not contain translation text",
            )
            .retryable(true)
        })
}

pub fn extract_error_message(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body)
        && let Some(message) = extract_error_message_value(&value)
    {
        return message;
    }
    body.chars().take(400).collect()
}

fn extract_error_message_value(value: &Value) -> Option<String> {
    value
        .get("error")
        .and_then(|error| {
            error
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| error.as_str())
        })
        .or_else(|| value.get("message").and_then(Value::as_str))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_sse_across_chunk_boundaries() {
        let mut decoder = SseDecoder::default();
        let first = decoder
            .push_chunk("data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n".as_bytes())
            .unwrap();
        assert!(first.is_empty());
        let second = decoder.push_chunk(b"\ndata: [DONE]\n\n").unwrap();
        assert_eq!(
            second,
            vec![OpenAiEvent::Delta("你".into()), OpenAiEvent::Done,]
        );
    }

    #[test]
    fn decodes_multibyte_text_and_crlf_split_at_every_byte() {
        let input = "data: {\"choices\":[{\"delta\":{\"content\":\"你好🌿\"}}]}\r\n\r\ndata: [DONE]\r\n\r\n";
        let mut decoder = SseDecoder::default();
        let mut events = Vec::new();
        for byte in input.as_bytes() {
            events.extend(decoder.push_chunk(std::slice::from_ref(byte)).unwrap());
        }
        events.extend(decoder.finish().unwrap());
        assert_eq!(
            events,
            vec![OpenAiEvent::Delta("你好🌿".into()), OpenAiEvent::Done]
        );
    }

    #[test]
    fn accepts_bom_and_carriage_return_line_endings() {
        let input =
            "\u{feff}data: {\"choices\":[{\"delta\":{\"content\":\"好\"}}]}\r\rdata: [DONE]\r\r";
        let mut decoder = SseDecoder::default();
        let mut events = Vec::new();
        for byte in input.as_bytes() {
            events.extend(decoder.push_chunk(std::slice::from_ref(byte)).unwrap());
        }
        events.extend(decoder.finish().unwrap());
        assert_eq!(
            events,
            vec![OpenAiEvent::Delta("好".into()), OpenAiEvent::Done]
        );
    }

    #[test]
    fn invalid_stream_payload_is_an_error_not_translation_text() {
        let mut decoder = SseDecoder::default();
        let error = decoder.push_chunk(b"data: {bad\n\n").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Protocol);
    }

    #[test]
    fn stream_error_object_is_reported() {
        let mut decoder = SseDecoder::default();
        let error = decoder
            .push_chunk(b"data: {\"error\":{\"message\":\"model unavailable\"}}\n\n")
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Protocol);
        assert_eq!(error.message, "model unavailable");
    }

    #[test]
    fn extracts_complete_message() {
        let body = r#"{"choices":[{"message":{"content":"你好"}}]}"#;
        assert_eq!(extract_full_content(body).unwrap(), "你好");
    }

    #[test]
    fn extracts_provider_error_message() {
        let body = r#"{"error":{"message":"invalid key"}}"#;
        assert_eq!(extract_error_message(body), "invalid key");
    }

    #[test]
    fn invalid_utf8_is_reported() {
        let mut decoder = SseDecoder::default();
        let error = decoder.push_chunk(b"data: \xff\n\n").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Protocol);
    }
}
