//! `Std.Encoding`: hex, base64 and percent-encoding, and checksums.

use super::{assert_error, errors};

/// A context that imports every name of `Std.Encoding` and uses each.
const CODES: &str = "context Codes {\n\
     import { to_hex, from_hex, to_base64, from_base64, percent_encode, percent_decode, crc16, crc32 } from Std.Encoding\n\
     fn hex(data: Bytes): String = data.to_hex()\n\
     fn bytes(text: String): Option<Bytes> = from_hex(text)\n\
     fn base64(data: Bytes): String = data.to_base64()\n\
     fn unbase64(text: String): Option<Bytes> = from_base64(text)\n\
     fn query(text: String): String = text.percent_encode()\n\
     fn unquery(text: String): Option<String> = text.percent_decode()\n\
     fn short_sum(data: Bytes): Int = data.crc16()\n\
     fn sum(data: Bytes): Int = data.crc32()\n\
     }";

#[test]
fn gives_each_name_its_type() {
    assert_eq!(errors(CODES), Vec::<String>::new());
}

#[test]
fn names_the_import_each_one_needs() {
    let source = CODES.replace(
        "import { to_hex, from_hex, to_base64, from_base64, percent_encode, percent_decode, crc16, crc32 } from Std.Encoding",
        "",
    );

    let expected: Vec<String> = [
        "to_hex",
        "from_hex",
        "to_base64",
        "from_base64",
        "percent_encode",
        "percent_decode",
        "crc16",
        "crc32",
    ]
    .iter()
    .map(|name| format!("`{name}` needs `import {{ {name} }} from Std.Encoding`"))
    .collect();

    assert_eq!(errors(&source), expected);
}

#[test]
fn checks_bytes_methods_against_their_own_context() {
    // `to_text` comes from Std.Text, so importing it from Std.Encoding
    // isn't enough, and `to_hex` on Bytes needs Std.Encoding.
    assert_error(
        "context Codes {\n import { to_text } from Std.Encoding\n}",
        "`Std.Encoding` does not provide `to_text`",
    );

    assert_error(
        "context Codes {\n import { to_text } from Std.Text\n fn f(data: Bytes): String = data.to_text() + data.to_hex()\n}",
        "`to_hex` needs `import { to_hex } from Std.Encoding`",
    );
}

#[test]
fn takes_text_to_decode_and_nothing_to_encode() {
    let source = "context Codes {\n import { from_hex, to_hex } from Std.Encoding\n fn f(data: Bytes): Option<Bytes> = from_hex(data)\n fn g(data: Bytes): String = data.to_hex(2)\n}";

    assert_eq!(
        errors(source),
        [
            "expected `String`, found `Bytes`",
            "`to_hex` takes 0 arguments, found 1",
        ]
    );
}

#[test]
fn keeps_the_name_of_an_encoding_function() {
    assert_error(
        "context Codes {\n import { from_hex as unhex } from Std.Encoding\n}",
        "`from_hex` is built in, so it can't be renamed",
    );
}

#[test]
fn gives_checksums_as_whole_numbers() {
    assert_error(
        "context Codes {\n import { crc32 } from Std.Encoding\n fn f(data: Bytes): String = data.crc32()\n}",
        "expected `String`, found `Int`",
    );
}

/// `from_hex` reads bytes when imported from Std.Encoding and a whole number
/// when imported from Std.Text: the import decides.
#[test]
fn reads_hex_as_the_import_says() {
    assert_eq!(
        errors(
            "context Codes {\n import { from_hex } from Std.Encoding\n fn f(text: String): Option<Bytes> = from_hex(text)\n}\n\
             context Numbers {\n import { from_hex } from Std.Text\n fn f(text: String): Option<Int> = from_hex(text)\n}"
        ),
        Vec::<String>::new()
    );

    assert_error(
        "context Codes {\n import { from_hex } from Std.Encoding\n fn f(text: String): Option<Int> = from_hex(text)\n}",
        "expected `Option<Int>`, found `Option<Bytes>`",
    );

    assert_error(
        "context Numbers {\n import { from_hex } from Std.Text\n fn f(text: String): Option<Bytes> = from_hex(text)\n}",
        "expected `Option<Bytes>`, found `Option<Int>`",
    );
}

/// One context can't have `from_hex` from both, since nothing in a call
/// says which it means.
#[test]
fn reports_from_hex_imported_from_both() {
    let expected = "`from_hex` can't come from both `Std.Text` and `Std.Encoding`: \
                    they mean different things, so import it from one";

    assert_error(
        "context Codes {\n import { from_hex } from Std.Text\n import { from_hex } from Std.Encoding\n}",
        expected,
    );

    assert_error(
        "context Codes {\n import { from_hex } from Std.Encoding\n import { trim } from Std.Text\n import { from_hex } from Std.Text\n}",
        "`from_hex` can't come from both `Std.Encoding` and `Std.Text`: \
         they mean different things, so import it from one",
    );
}

/// `to_hex` is a method, so the receiver picks: both imports are fine
/// together, and each type needs its own.
#[test]
fn picks_to_hex_by_the_receiver() {
    assert_eq!(
        errors(
            "context Codes {\n import { to_hex } from Std.Text\n import { to_hex } from Std.Encoding\n\
             fn f(data: Bytes, number: Int): String = data.to_hex() + number.to_hex()\n}"
        ),
        Vec::<String>::new()
    );

    assert_error(
        "context Codes {\n import { to_hex } from Std.Encoding\n fn f(number: Int): String = number.to_hex()\n}",
        "`to_hex` needs `import { to_hex } from Std.Text`",
    );
}

/// With no import and nothing to say which is meant, both are named.
#[test]
fn names_both_imports_from_hex_could_need() {
    assert_error(
        "context Codes {\n fn f(text: String): Bool = from_hex(text) == None\n}",
        "`from_hex` needs `import { from_hex } from Std.Text` or `import { from_hex } from Std.Encoding`",
    );
}
