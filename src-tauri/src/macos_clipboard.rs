//! Preserve macOS pasteboard items, including legacy format aliases.

use crate::capture::ClipboardItem;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};
use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

type Contents = [(String, Vec<u8>)];
type Items = [ClipboardItem];

struct TemporaryPasteboard(Retained<NSPasteboard>);

impl Drop for TemporaryPasteboard {
    fn drop(&mut self) {
        self.0.clearContents();
    }
}

fn preservation_error() -> YiyuError {
    YiyuError::new(
        ErrorKind::Capture,
        "clipboard formats cannot be safely preserved",
    )
}

fn write(board: &NSPasteboard, contents: &Contents) -> YiyuResult<()> {
    if contents.is_empty() {
        board.clearContents();
        return Ok(());
    }
    let types: Vec<_> = contents
        .iter()
        .map(|(format, _)| NSString::from_str(format))
        .collect();
    let declared = NSArray::from_retained_slice(&types);
    // SAFETY: A nil owner is valid. All declared types are NSStrings owned by the array.
    unsafe { board.declareTypes_owner(&declared, None) };
    for ((_, bytes), format) in contents.iter().zip(types.iter()) {
        if !board.setData_forType(Some(&NSData::with_bytes(bytes)), format) {
            return Err(preservation_error());
        }
    }
    Ok(())
}

fn read(board: &NSPasteboard) -> YiyuResult<Vec<(String, Vec<u8>)>> {
    let types = board.types().ok_or_else(preservation_error)?;
    types
        .iter()
        .map(|format| {
            let data = board.dataForType(&format).ok_or_else(preservation_error)?;
            Ok((format.to_string(), data.to_vec()))
        })
        .collect()
}

pub fn validate(contents: &Contents) -> YiyuResult<()> {
    if contents.is_empty() {
        return Ok(());
    }
    // A temporary uniquely named pasteboard checks that macOS can round-trip every format
    // before the general clipboard is touched. It is cleared on both paths.
    let board = TemporaryPasteboard(NSPasteboard::pasteboardWithUniqueName());
    write(&board.0, contents)?;
    let restored = read(&board.0)?;
    // AppKit may synthesize additional aliases; every original representation
    // must still be present with exactly the same bytes.
    if !contents.iter().all(|entry| restored.contains(entry)) {
        return Err(preservation_error());
    }
    Ok(())
}

pub fn restore(contents: &Contents) -> YiyuResult<()> {
    write(&NSPasteboard::generalPasteboard(), contents)
}

fn read_items(board: &NSPasteboard) -> YiyuResult<Vec<ClipboardItem>> {
    let items = board.pasteboardItems().ok_or_else(preservation_error)?;
    items
        .iter()
        .map(|item| {
            let types = item.types();
            if types.is_empty() {
                return Err(preservation_error());
            }
            types
                .iter()
                .map(|format| {
                    let data = item.dataForType(&format).ok_or_else(preservation_error)?;
                    Ok((format.to_string(), data.to_vec()))
                })
                .collect()
        })
        .collect()
}

fn write_items(board: &NSPasteboard, items: &Items) -> YiyuResult<()> {
    let mut prepared = Vec::with_capacity(items.len());
    for representations in items {
        let item = NSPasteboardItem::new();
        for (format, bytes) in representations {
            if !item.setData_forType(&NSData::with_bytes(bytes), &NSString::from_str(format)) {
                return Err(preservation_error());
            }
        }
        prepared.push(item);
    }
    let objects: Vec<&ProtocolObject<dyn NSPasteboardWriting>> = prepared
        .iter()
        .map(|item| ProtocolObject::from_ref(&**item))
        .collect();
    let array = NSArray::from_slice(&objects);
    board.clearContents();
    if !board.writeObjects(&array) {
        return Err(preservation_error());
    }
    Ok(())
}

pub fn snapshot_items() -> YiyuResult<Vec<ClipboardItem>> {
    read_items(&NSPasteboard::generalPasteboard())
}

pub fn validate_items(items: &Items) -> YiyuResult<()> {
    let board = TemporaryPasteboard(NSPasteboard::pasteboardWithUniqueName());
    write_items(&board.0, items)?;
    let restored = read_items(&board.0)?;
    if restored.len() != items.len()
        || !items
            .iter()
            .zip(&restored)
            .all(|(expected, actual)| expected.iter().all(|entry| actual.contains(entry)))
    {
        return Err(preservation_error());
    }
    Ok(())
}

pub fn restore_items(items: &Items) -> YiyuResult<()> {
    write_items(&NSPasteboard::generalPasteboard(), items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_html_and_text_round_trip_on_isolated_pasteboards() {
        let source = NSPasteboard::pasteboardWithUniqueName();
        let destination = NSPasteboard::pasteboardWithUniqueName();
        let source_contents = vec![
            ("public.utf8-plain-text".into(), b"test text".to_vec()),
            (
                "Apple HTML pasteboard type".into(),
                b"<b>test text</b>".to_vec(),
            ),
        ];
        write(&source, &source_contents).unwrap();
        let original = read(&source).unwrap();
        assert!(
            original
                .iter()
                .any(|(format, _)| format == "Apple HTML pasteboard type")
        );
        assert!(
            original
                .iter()
                .any(|(format, _)| format == "NSStringPboardType")
        );
        validate(&original).unwrap();
        write(&destination, &original).unwrap();
        let mut restored = read(&destination).unwrap();
        let mut expected = original;
        restored.sort_unstable();
        expected.sort_unstable();
        assert_eq!(restored, expected);
        source.clearContents();
        destination.clearContents();
    }

    #[test]
    fn incompatible_alias_data_is_rejected_before_general_clipboard_changes() {
        let incompatible = vec![
            ("public.html".into(), b"first".to_vec()),
            ("Apple HTML pasteboard type".into(), b"different".to_vec()),
        ];
        assert!(validate(&incompatible).is_err());
    }

    #[test]
    fn multiple_items_round_trip_without_merging() {
        let source = TemporaryPasteboard(NSPasteboard::pasteboardWithUniqueName());
        let destination = TemporaryPasteboard(NSPasteboard::pasteboardWithUniqueName());
        let items = vec![
            vec![
                ("public.utf8-plain-text".into(), b"first".to_vec()),
                ("public.html".into(), b"<b>first</b>".to_vec()),
            ],
            vec![("public.utf8-plain-text".into(), b"second".to_vec())],
        ];
        write_items(&source.0, &items).unwrap();
        let captured = read_items(&source.0).unwrap();
        assert_eq!(captured.len(), 2);
        validate_items(&captured).unwrap();
        write_items(&destination.0, &captured).unwrap();
        let restored = read_items(&destination.0).unwrap();
        assert_eq!(restored, captured);
    }
}
