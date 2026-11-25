//! Converting between b's positions and an editor's, and between paths and URIs.
//!
//! b counts lines and columns from 1, a column being one character. The
//! language server protocol counts from 0, a column being one UTF-16 unit.

use std::path::{Path, PathBuf};

use b_syntax::Span;
use lsp_types::{Position, Range, Uri};

fn line_text(text: &str, line: u32) -> &str {
    text.lines().nth(line as usize).unwrap_or("")
}

/// The editor position of a b line and column.
pub fn to_position(text: &str, line: u32, column: u32) -> Position {
    let line = line.saturating_sub(1);
    let units: usize = line_text(text, line)
        .chars()
        .take(column.saturating_sub(1) as usize)
        .map(char::len_utf16)
        .sum();
    Position::new(line, units as u32)
}

/// The b line and column of an editor position.
pub fn from_position(text: &str, position: Position) -> (u32, u32) {
    let mut units = 0;
    let mut column = 1;
    for c in line_text(text, position.line).chars() {
        if units >= position.character as usize {
            break;
        }
        units += c.len_utf16();
        column += 1;
    }
    (position.line + 1, column)
}

/// The range of a name `length` characters long, written at `span`.
pub fn name_range(text: &str, span: Span, length: usize) -> Range {
    let start = to_position(text, span.line, span.column);
    let end = to_position(text, span.line, span.column + length as u32);
    Range::new(start, end)
}

/// The range of the word at `span`, or of its one character when it is not
/// in a word. A diagnostic only knows where it starts, so this is what it underlines.
pub fn word_range(text: &str, span: Span) -> Range {
    let line = span.line.max(1);
    let rest: Vec<char> = line_text(text, line - 1)
        .chars()
        .skip(span.column.saturating_sub(1) as usize)
        .collect();
    let is_word = |c: &char| c.is_alphanumeric() || *c == '_';
    let length = match rest.first() {
        Some(c) if is_word(c) => rest.iter().take_while(|c| is_word(c)).count(),
        Some(_) => 1,
        None => 0,
    };
    let span = Span { line, ..span };
    name_range(text, span, length)
}

/// A `file://` URI for a path, with anything but letters, digits, `/` and
/// `-._~` percent-encoded.
pub fn path_to_uri(path: &Path) -> Option<Uri> {
    let mut uri = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                uri.push(byte as char)
            }
            _ => uri.push_str(&format!("%{byte:02X}")),
        }
    }
    uri.parse().ok()
}

/// The path of a `file://` URI.
pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    let encoded = uri.as_str().strip_prefix("file://")?.as_bytes();
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut index = 0;
    while index < encoded.len() {
        let byte = encoded[index];
        let hex = encoded.get(index + 1..index + 3);
        let decoded = hex
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match decoded {
            Some(decoded) if byte == b'%' => {
                bytes.push(decoded);
                index += 3;
            }
            _ => {
                bytes.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(bytes).ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use b_syntax::SourceId;

    #[test]
    fn columns_are_characters_in_b_and_utf16_units_in_the_editor() {
        let text = "let é = 1\nlet 🙂x = 2";
        assert_eq!(to_position(text, 2, 6), Position::new(1, 6));
        assert_eq!(from_position(text, Position::new(1, 6)), (2, 6));
        assert_eq!(from_position(text, Position::new(0, 5)), (1, 6));
    }

    #[test]
    fn underlines_the_word_a_diagnostic_starts_at() {
        let span = Span {
            source: SourceId(0),
            line: 1,
            column: 5,
        };
        let range = word_range("let room_id = 3", span);
        assert_eq!((range.start.character, range.end.character), (4, 11));
    }

    #[test]
    fn paths_survive_a_round_trip_through_uris() {
        let path = Path::new("/Users/me/My Code/dungeon #1/room.b");
        let uri = path_to_uri(path).expect("a valid URI");
        assert_eq!(
            uri.as_str(),
            "file:///Users/me/My%20Code/dungeon%20%231/room.b"
        );
        assert_eq!(uri_to_path(&uri).as_deref(), Some(path));
    }
}
