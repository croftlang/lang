//! Converting between Croft's positions and an editor's, and between paths and URIs.
//!
//! Croft counts lines and columns from 1, a column being one character. The
//! language server protocol counts from 0, a column being one UTF-16 unit.

use std::path::{Path, PathBuf};

use croft_syntax::Span;
use lsp_types::{Position, Range, Uri};

fn line_text(text: &str, line: u32) -> &str {
    text.lines().nth(line as usize).unwrap_or("")
}

/// The editor position of a Croft line and column.
pub fn to_position(text: &str, line: u32, column: u32) -> Position {
    let line = line.saturating_sub(1);

    let units: usize = line_text(text, line)
        .chars()
        .take(column.saturating_sub(1) as usize)
        .map(char::len_utf16)
        .sum();

    Position::new(line, units as u32)
}

/// The Croft line and column of an editor position.
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
/// `-._~` percent-encoded. A Windows path such as `C:\code\a.croft` becomes
/// `file:///C:/code/a.croft`.
pub fn path_to_uri(path: &Path) -> Option<Uri> {
    text_to_uri(&path.to_string_lossy(), cfg!(windows))?
        .parse()
        .ok()
}

fn text_to_uri(path: &str, windows: bool) -> Option<String> {
    let mut path = path.to_string();

    if windows {
        path = path.replace('\\', "/");

        if let Some(share) = path.strip_prefix("//") {
            // `\\server\share\a.croft` is `file://server/share/a.croft`.
            let (host, rest) = share.split_once('/')?;
            return Some(format!("file://{host}{}", encode(&format!("/{rest}"))));
        }

        if is_drive(&path) {
            path.insert(0, '/');
        }
    }

    Some(format!("file://{}", encode(&path)))
}

/// Does a path start with a drive letter, as in `C:/code`?
fn is_drive(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn encode(path: &str) -> String {
    let mut encoded = String::new();

    for (index, byte) in path.bytes().enumerate() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                encoded.push(byte as char)
            }
            // The colon after a drive letter, as in `/C:/`.
            b':' if index == 2 && path.starts_with('/') && is_drive(&path[1..]) => {
                encoded.push(':')
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }

    encoded
}

/// The path of a `file://` URI.
pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    uri_text_to_path(uri.as_str(), cfg!(windows)).map(PathBuf::from)
}

fn uri_text_to_path(uri: &str, windows: bool) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;

    // Everything before the path is the host, empty for this machine.
    let (host, path) = match rest.find('/') {
        Some(slash) => rest.split_at(slash),
        None => (rest, ""),
    };

    let path = decode(path)?;

    if !windows {
        return (host.is_empty() || host == "localhost").then_some(path);
    }

    // `/c%3A/code/a.croft`, as VS Code writes it, is `C:\code\a.croft`. The drive
    // is written in capitals, as Windows writes it, so the same file always
    // has the same path.
    let path = match path.strip_prefix('/') {
        Some(rest) if is_drive(rest) => rest[..1].to_ascii_uppercase() + &rest[1..],
        _ => path,
    };

    let path = path.replace('/', "\\");

    match host {
        "" | "localhost" => Some(path),
        host => Some(format!("\\\\{host}{path}")),
    }
}

fn decode(encoded: &str) -> Option<String> {
    let encoded = encoded.as_bytes();
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

    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use croft_syntax::SourceId;

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
        let path = Path::new("/Users/me/My Code/dungeon #1/room.croft");
        let uri = path_to_uri(path).expect("a valid URI");

        assert_eq!(
            uri.as_str(),
            "file:///Users/me/My%20Code/dungeon%20%231/room.croft"
        );

        assert_eq!(uri_to_path(&uri).as_deref(), Some(path));
    }

    #[test]
    fn windows_paths_have_a_drive_or_a_server() {
        let uri = text_to_uri(r"C:\Users\me\My Code\room.croft", true).expect("a URI");
        assert_eq!(uri, "file:///C:/Users/me/My%20Code/room.croft");

        let path = uri_text_to_path(&uri, true).expect("a path");
        assert_eq!(path, r"C:\Users\me\My Code\room.croft");

        // VS Code encodes the colon and writes the drive in lower case.
        let path = uri_text_to_path("file:///c%3A/code/room.croft", true).expect("a path");
        assert_eq!(path, r"C:\code\room.croft");

        let uri = text_to_uri(r"\\server\share\room.croft", true).expect("a URI");
        assert_eq!(uri, "file://server/share/room.croft");

        let path = uri_text_to_path(&uri, true).expect("a path");
        assert_eq!(path, r"\\server\share\room.croft");
    }
}
