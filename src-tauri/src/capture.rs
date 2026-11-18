use clipboard_rs::{Clipboard, ClipboardContent, ClipboardContext};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::future::Future;
use std::time::Duration;
use tokio::time::{Instant, sleep};
use tokio_util::sync::CancellationToken;
use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

static CLIPBOARD_ACCESS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn with_clipboard_access<F, Fut>(operation: F) -> Fut::Output
where
    F: FnOnce() -> Fut,
    Fut: Future,
{
    let _guard = CLIPBOARD_ACCESS.lock().await;
    operation().await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureError {
    pub kind: CaptureFailure,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFailure {
    Permission,
    EmptySelection,
    Unsupported,
    Clipboard,
    Cancelled,
}

impl From<CaptureError> for YiyuError {
    fn from(value: CaptureError) -> Self {
        let kind = match value.kind {
            CaptureFailure::Permission => ErrorKind::Permission,
            CaptureFailure::EmptySelection => ErrorKind::Capture,
            CaptureFailure::Unsupported => ErrorKind::Unsupported,
            CaptureFailure::Clipboard => ErrorKind::Capture,
            CaptureFailure::Cancelled => ErrorKind::Cancelled,
        };
        let mut error = YiyuError::new(kind, value.message);
        if matches!(
            value.kind,
            CaptureFailure::Permission | CaptureFailure::Unsupported | CaptureFailure::Clipboard
        ) {
            error.actions.push("open-settings".into());
        }
        error
    }
}

pub fn permission_status() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        PermissionStatus {
            accessibility: axuielement::is_process_trusted(),
            screen_recording: true,
        }
    }
    #[cfg(target_os = "windows")]
    {
        PermissionStatus {
            accessibility: true,
            screen_recording: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    pub accessibility: bool,
    pub screen_recording: bool,
}

pub fn request_accessibility_permission() {
    #[cfg(target_os = "macos")]
    {
        if !axuielement::is_process_trusted() {
            // Ask macOS to register this exact app; the system prompt opens its settings.
            // Opening settings separately leaves a duplicate system dialog behind it.
            let _ = axuielement::is_process_trusted_with_prompt();
        }
    }
}

pub async fn capture_selection(
    use_fallback: bool,
    cancellation: Option<&CancellationToken>,
) -> Result<String, CaptureError> {
    let permissions = permission_status();
    if !permissions.accessibility {
        return Err(CaptureError {
            kind: CaptureFailure::Permission,
            message: "Accessibility permission is required to read selected text".into(),
        });
    }

    let native_failure = match read_native_selection() {
        Ok(Some(text)) => {
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
            None
        }
        Ok(None) => None,
        Err(error) => Some(error),
    };

    if !use_fallback {
        return Err(native_failure.unwrap_or(CaptureError {
            kind: CaptureFailure::Unsupported,
            message: "No readable selection; conservative mode prevents clipboard fallback".into(),
        }));
    }

    if native_failure
        .as_ref()
        .is_some_and(|error| !should_try_clipboard_after_native(error))
    {
        return Err(native_failure.expect("checked above"));
    }

    capture_selection_via_clipboard(cancellation).await
}

fn should_try_clipboard_after_native(error: &CaptureError) -> bool {
    error.kind != CaptureFailure::Permission
}

pub async fn read_clipboard_text() -> YiyuResult<String> {
    with_clipboard_access(|| async {
        let context = ClipboardContext::new().map_err(clipboard_error)?;
        let text = context.get_text().map_err(clipboard_error)?;
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            return Err(YiyuError::new(ErrorKind::Capture, "clipboard is empty").field("clipboard"));
        }
        Ok(trimmed)
    })
    .await
}

pub async fn copy_text(text: &str) -> YiyuResult<()> {
    with_clipboard_access(|| async {
        let context = ClipboardContext::new().map_err(clipboard_error)?;
        context
            .set_text(text.to_string())
            .map_err(clipboard_error)?;
        Ok(())
    })
    .await
}

async fn capture_selection_via_clipboard(
    cancellation: Option<&CancellationToken>,
) -> Result<String, CaptureError> {
    with_cancellable_clipboard_access(cancellation, capture_selection_via_clipboard_locked).await?
}

async fn with_cancellable_clipboard_access<F, Fut>(
    cancellation: Option<&CancellationToken>,
    operation: F,
) -> Result<Fut::Output, CaptureError>
where
    F: FnOnce() -> Fut,
    Fut: Future,
{
    let _guard = match cancellation {
        Some(token) => tokio::select! {
            biased;
            _ = token.cancelled() => return Err(CaptureError {
                kind: CaptureFailure::Cancelled,
                message: "selection capture cancelled".into(),
            }),
            guard = CLIPBOARD_ACCESS.lock() => guard,
        },
        None => CLIPBOARD_ACCESS.lock().await,
    };
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        return Err(CaptureError {
            kind: CaptureFailure::Cancelled,
            message: "selection capture cancelled".into(),
        });
    }
    // Once clipboard simulation begins, finish its restoration even if superseded.
    Ok(operation().await)
}

async fn capture_selection_via_clipboard_locked() -> Result<String, CaptureError> {
    let context = ClipboardContext::new().map_err(|error| CaptureError {
        kind: CaptureFailure::Clipboard,
        message: format!("cannot access clipboard: {error}"),
    })?;
    let snapshot = snapshot_clipboard(&context).map_err(|error| CaptureError {
        kind: CaptureFailure::Clipboard,
        message: format!("cannot preserve current clipboard: {}", error.message),
    })?;
    if !snapshot.contents.is_empty()
        && !snapshot
            .contents
            .iter()
            .any(|(format, bytes)| !bytes.is_empty() && is_restorable_format(format))
    {
        return Err(CaptureError {
            kind: CaptureFailure::Clipboard,
            message: "current clipboard formats cannot be preserved".into(),
        });
    }
    let mut restore_guard = ClipboardRestoreGuard::new(&context, &snapshot);

    let sentinel = format!("__YIYU_{}__", uuid::Uuid::new_v4());
    if let Err(error) = context.set_text(sentinel.clone()) {
        return Err(CaptureError {
            kind: CaptureFailure::Clipboard,
            message: format!("cannot prepare clipboard fallback: {error}"),
        });
    }
    let sentinel_fingerprint = clipboard_fingerprint(&context).ok();
    if context.get_text().ok().as_deref() != Some(sentinel.as_str()) {
        return Err(CaptureError {
            kind: CaptureFailure::Clipboard,
            message: "clipboard changed while preparing selection".into(),
        });
    }
    restore_guard.track(sentinel_fingerprint);

    if let Err(error) = simulate_copy() {
        return Err(CaptureError {
            kind: CaptureFailure::Clipboard,
            message: format!("cannot simulate copy: {error}"),
        });
    }

    let deadline = Instant::now() + Duration::from_millis(800);
    let mut selected = None;
    while Instant::now() < deadline {
        sleep(Duration::from_millis(20)).await;
        if let Ok(text) = context.get_text()
            && text != sentinel
            && !text.trim().is_empty()
        {
            selected = Some(text);
            break;
        }
    }

    let selected = selected.ok_or_else(|| CaptureError {
        kind: CaptureFailure::EmptySelection,
        message: "No text was detected in the current selection".into(),
    })?;

    let post_copy_fingerprint = clipboard_fingerprint(&context).ok();
    if context.get_text().ok().as_deref() == Some(selected.as_str()) {
        restore_guard.track(post_copy_fingerprint);
    } else {
        restore_guard.disarm();
    }
    // Do not restore over a newer clipboard value from another application.
    sleep(Duration::from_millis(30)).await;
    restore_guard.restore().map_err(|error| CaptureError {
        kind: CaptureFailure::Clipboard,
        message: format!("cannot restore clipboard: {}", error.message),
    })?;

    let trimmed = selected.trim().to_string();
    if trimmed.is_empty() {
        return Err(CaptureError {
            kind: CaptureFailure::EmptySelection,
            message: "No text was detected in the current selection".into(),
        });
    }
    Ok(trimmed)
}

#[derive(Clone, Debug)]
struct ClipboardSnapshot {
    contents: Vec<(String, Vec<u8>)>,
}

fn snapshot_clipboard(context: &ClipboardContext) -> YiyuResult<ClipboardSnapshot> {
    let formats = context.available_formats().map_err(clipboard_error)?;
    let mut contents = Vec::with_capacity(formats.len());
    for format in formats {
        let bytes = context.get_buffer(&format).map_err(clipboard_error)?;
        contents.push((format, bytes));
    }
    Ok(ClipboardSnapshot { contents })
}

fn restore_clipboard(context: &ClipboardContext, snapshot: &ClipboardSnapshot) -> YiyuResult<()> {
    if snapshot.contents.is_empty() {
        return context.clear().map_err(clipboard_error);
    }
    let contents: Vec<ClipboardContent> = snapshot
        .contents
        .iter()
        .filter(|(format, bytes)| !bytes.is_empty() && is_restorable_format(format))
        .map(|(format, bytes)| ClipboardContent::Other(format.clone(), bytes.clone()))
        .collect();
    if contents.is_empty() {
        return Err(YiyuError::new(
            ErrorKind::Capture,
            "clipboard formats cannot be restored",
        ));
    }
    context.set(contents).map_err(clipboard_error)
}

trait ClipboardRestore {
    fn fingerprint(&self) -> YiyuResult<String>;
    fn restore_snapshot(&self, snapshot: &ClipboardSnapshot) -> YiyuResult<()>;
}

impl ClipboardRestore for ClipboardContext {
    fn fingerprint(&self) -> YiyuResult<String> {
        clipboard_fingerprint(self)
    }

    fn restore_snapshot(&self, snapshot: &ClipboardSnapshot) -> YiyuResult<()> {
        restore_clipboard(self, snapshot)
    }
}

struct ClipboardRestoreGuard<'a, C: ClipboardRestore> {
    context: &'a C,
    snapshot: &'a ClipboardSnapshot,
    expected: Option<String>,
}

impl<'a, C: ClipboardRestore> ClipboardRestoreGuard<'a, C> {
    fn new(context: &'a C, snapshot: &'a ClipboardSnapshot) -> Self {
        Self {
            context,
            snapshot,
            expected: None,
        }
    }

    fn track(&mut self, fingerprint: Option<String>) {
        self.expected = fingerprint;
    }

    fn restore(&mut self) -> YiyuResult<()> {
        let Some(expected) = self.expected.take() else {
            return Ok(());
        };
        if clipboard_is_unchanged(Some(&expected), self.context.fingerprint().ok().as_deref()) {
            self.context.restore_snapshot(self.snapshot)?;
        }
        Ok(())
    }

    fn disarm(&mut self) {
        self.expected = None;
    }
}

impl<C: ClipboardRestore> Drop for ClipboardRestoreGuard<'_, C> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn is_restorable_format(format: &str) -> bool {
    !(cfg!(target_os = "macos") && format.ends_with("PboardType"))
}

fn clipboard_fingerprint(context: &ClipboardContext) -> YiyuResult<String> {
    let snapshot = snapshot_clipboard(context)?;
    Ok(contents_fingerprint(&snapshot.contents))
}

fn contents_fingerprint(contents: &[(String, Vec<u8>)]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let mut ordered = contents.iter().collect::<Vec<_>>();
    ordered.sort_unstable();
    for (format, bytes) in ordered {
        hasher.update((format.len() as u64).to_be_bytes());
        hasher.update(format.as_bytes());
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    format!("{:x}", hasher.finalize())
}

fn clipboard_is_unchanged(previous: Option<&str>, current: Option<&str>) -> bool {
    previous
        .zip(current)
        .is_some_and(|(previous, current)| previous == current)
}

fn simulate_copy() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|error| error.to_string())?;
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(target_os = "windows")]
    let modifier = Key::Control;

    enigo
        .key(modifier, Direction::Press)
        .map_err(|error| error.to_string())?;
    let result = enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|error| error.to_string());
    let release = enigo
        .key(modifier, Direction::Release)
        .map_err(|error| error.to_string());
    result.and(release)
}

fn clipboard_error(error: impl std::fmt::Display) -> YiyuError {
    YiyuError::new(ErrorKind::Capture, format!("clipboard error: {error}")).action("open-settings")
}

#[cfg(target_os = "macos")]
fn read_native_selection() -> Result<Option<String>, CaptureError> {
    use axuielement::prelude::*;

    let system = system_wide().ok_or_else(|| CaptureError {
        kind: CaptureFailure::Permission,
        message: "system accessibility object is unavailable".into(),
    })?;
    let focused = system.focused_ui_element().map_err(|error| CaptureError {
        kind: CaptureFailure::Unsupported,
        message: format!("cannot inspect focused UI element: {error}"),
    })?;
    let Some(focused) = focused else {
        return Ok(None);
    };
    let selected = focused
        .string_attribute(axuielement::ax_attribute::AX_SELECTED_TEXT_ATTRIBUTE)
        .map_err(|error| CaptureError {
            kind: CaptureFailure::Unsupported,
            message: format!("cannot read AXSelectedText: {error}"),
        })?;
    Ok(selected.filter(|text| !text.trim().is_empty()))
}

#[cfg(target_os = "windows")]
fn read_native_selection() -> Result<Option<String>, CaptureError> {
    use uiautomation::UIAutomation;
    use uiautomation::patterns::UITextPattern;

    let automation = UIAutomation::new().map_err(|error| CaptureError {
        kind: CaptureFailure::Unsupported,
        message: format!("cannot initialize UI Automation: {error}"),
    })?;
    let focused = automation
        .get_focused_element()
        .map_err(|error| CaptureError {
            kind: CaptureFailure::Unsupported,
            message: format!("cannot read focused element: {error}"),
        })?;
    let pattern = focused
        .get_pattern::<UITextPattern>()
        .map_err(|error| CaptureError {
            kind: CaptureFailure::Unsupported,
            message: format!("focused element has no text selection pattern: {error}"),
        })?;
    let ranges = pattern.get_selection().map_err(|error| CaptureError {
        kind: CaptureFailure::Unsupported,
        message: format!("cannot read selected text ranges: {error}"),
    })?;
    let mut texts = Vec::new();
    for range in ranges {
        if let Ok(text) = range.get_text(-1)
            && !text.trim().is_empty()
        {
            texts.push(text);
        }
    }
    Ok((!texts.is_empty()).then(|| texts.join("\n")))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn read_native_selection() -> Result<Option<String>, CaptureError> {
    Err(CaptureError {
        kind: CaptureFailure::Unsupported,
        message: "unsupported platform".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn clipboard_operations_wait_for_the_previous_operation_to_finish() {
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let first = tokio::spawn(with_clipboard_access(|| async move {
            entered_tx.send(()).unwrap();
            release_rx.await.unwrap();
        }));
        entered_rx.await.unwrap();

        let (second_tx, mut second_rx) = tokio::sync::oneshot::channel();
        let second = tokio::spawn(with_clipboard_access(|| async move {
            second_tx.send(()).unwrap();
        }));
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut second_rx)
                .await
                .is_err()
        );
        release_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), second_rx)
            .await
            .unwrap()
            .unwrap();
        first.await.unwrap();
        second.await.unwrap();
    }

    #[tokio::test]
    async fn superseded_capture_finishes_restoration_before_next_clipboard_read() {
        let clipboard = std::sync::Arc::new(std::sync::Mutex::new("original"));
        let first_clipboard = clipboard.clone();
        let cancellation = CancellationToken::new();
        let first_cancellation = cancellation.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let first = tokio::spawn(async move {
            with_cancellable_clipboard_access(Some(&first_cancellation), || async move {
                *first_clipboard.lock().unwrap() = "temporary";
                entered_tx.send(()).unwrap();
                release_rx.await.unwrap();
                *first_clipboard.lock().unwrap() = "original";
            })
            .await
        });
        entered_rx.await.unwrap();
        cancellation.cancel();

        let next_clipboard = clipboard.clone();
        let (read_tx, mut read_rx) = tokio::sync::oneshot::channel();
        let next = tokio::spawn(with_clipboard_access(|| async move {
            read_tx.send(*next_clipboard.lock().unwrap()).unwrap();
        }));
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut read_rx)
                .await
                .is_err()
        );
        release_tx.send(()).unwrap();
        first.await.unwrap().unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), read_rx)
                .await
                .unwrap()
                .unwrap(),
            "original"
        );
        next.await.unwrap();
    }

    #[tokio::test]
    async fn cancelled_capture_does_not_enter_the_clipboard_operation() {
        let _guard = CLIPBOARD_ACCESS.lock().await;
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let result = with_cancellable_clipboard_access(Some(&cancellation), || async {
            panic!("cancelled capture must not touch the clipboard");
        })
        .await;
        assert_eq!(result.unwrap_err().kind, CaptureFailure::Cancelled);
    }

    #[derive(Default)]
    struct FakeClipboard {
        fingerprint: std::cell::RefCell<String>,
        restores: std::cell::Cell<usize>,
        restored_empty: std::cell::Cell<bool>,
    }

    impl ClipboardRestore for FakeClipboard {
        fn fingerprint(&self) -> YiyuResult<String> {
            Ok(self.fingerprint.borrow().clone())
        }

        fn restore_snapshot(&self, snapshot: &ClipboardSnapshot) -> YiyuResult<()> {
            self.restores.set(self.restores.get() + 1);
            self.restored_empty.set(snapshot.contents.is_empty());
            Ok(())
        }
    }

    #[test]
    fn guard_preserves_new_user_clipboard_on_early_exit() {
        let clipboard = FakeClipboard::default();
        let original = ClipboardSnapshot {
            contents: Vec::new(),
        };
        *clipboard.fingerprint.borrow_mut() = "sentinel".into();
        {
            let mut guard = ClipboardRestoreGuard::new(&clipboard, &original);
            guard.track(Some("sentinel".into()));
            *clipboard.fingerprint.borrow_mut() = "new user copy".into();
        }
        assert_eq!(clipboard.restores.get(), 0);
        assert_eq!(&*clipboard.fingerprint.borrow(), "new user copy");
    }

    #[test]
    fn guard_restores_an_initially_empty_clipboard_when_unchanged() {
        let clipboard = FakeClipboard::default();
        let original = ClipboardSnapshot {
            contents: Vec::new(),
        };
        *clipboard.fingerprint.borrow_mut() = "sentinel".into();
        {
            let mut guard = ClipboardRestoreGuard::new(&clipboard, &original);
            guard.track(Some("sentinel".into()));
        }
        assert_eq!(clipboard.restores.get(), 1);
        assert!(clipboard.restored_empty.get());
    }

    #[test]
    fn capture_error_maps_permission_to_action() {
        let error: YiyuError = CaptureError {
            kind: CaptureFailure::Permission,
            message: "permission".into(),
        }
        .into();
        assert_eq!(error.kind, ErrorKind::Permission);
        assert!(error.actions.contains(&"open-settings".to_string()));
    }

    #[test]
    fn unsupported_native_reading_falls_through_to_clipboard() {
        let error = CaptureError {
            kind: CaptureFailure::Unsupported,
            message: "native reader unavailable".into(),
        };
        assert!(should_try_clipboard_after_native(&error));
    }

    #[test]
    fn permission_failure_does_not_try_clipboard() {
        let error = CaptureError {
            kind: CaptureFailure::Permission,
            message: "permission missing".into(),
        };
        assert!(!should_try_clipboard_after_native(&error));
    }

    #[test]
    fn unknown_fingerprints_do_not_overwrite_the_clipboard() {
        assert!(clipboard_is_unchanged(Some("same"), Some("same")));
        assert!(!clipboard_is_unchanged(Some("old"), Some("new")));
        assert!(!clipboard_is_unchanged(None, Some("new")));
        assert!(!clipboard_is_unchanged(Some("old"), None));
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let first = ClipboardSnapshot {
            contents: vec![("text/plain".into(), b"hello".to_vec())],
        };
        let second = ClipboardSnapshot {
            contents: vec![("text/plain".into(), b"hello".to_vec())],
        };
        assert_eq!(
            contents_fingerprint(&first.contents),
            contents_fingerprint(&second.contents)
        );
    }

    #[test]
    fn clipboard_fingerprint_ignores_format_order_and_separates_fields() {
        let first = vec![("a".into(), b"bc".to_vec()), ("x".into(), b"y".to_vec())];
        let reversed = vec![("x".into(), b"y".to_vec()), ("a".into(), b"bc".to_vec())];
        let different = vec![("ab".into(), b"c".to_vec()), ("x".into(), b"y".to_vec())];
        assert_eq!(
            contents_fingerprint(&first),
            contents_fingerprint(&reversed)
        );
        assert_ne!(
            contents_fingerprint(&first),
            contents_fingerprint(&different)
        );
    }

    #[test]
    fn legacy_macos_pasteboard_aliases_are_not_replayed() {
        assert!(is_restorable_format("public.utf8-plain-text"));
        if cfg!(target_os = "macos") {
            assert!(!is_restorable_format("NSStringPboardType"));
        }
    }
}
