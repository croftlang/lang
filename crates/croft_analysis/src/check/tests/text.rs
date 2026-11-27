//! Std.Text's character codes, hexadecimal and decimal text.

use super::{assert_error, errors};

/// A context importing the names under test, with `body` among its functions.
fn text(body: &str) -> String {
    format!(
        "context Shop {{\n\
         import {{ code_at, from_code, to_hex, from_hex, to_decimal }} from Std.Text\n\
         import {{ Decimal }} from Std.Numbers\n\
         {body}\n}}"
    )
}

#[test]
fn types_character_codes_hexadecimal_and_decimal_text() {
    let source = text(
        "fn a(name: String): Option<Int> = name.code_at(0)\n\
         fn b(code: Int): String = from_code(code)\n\
         fn c(number: Int): String = number.to_hex()\n\
         fn d(written: String): Option<Int> = from_hex(written)\n\
         fn e(written: String): Option<Decimal<2>> = written.to_decimal()\n\
         fn f(written: String): Decimal<3> = {\n\
             let price: Option<Decimal<3>> = written.to_decimal()\n\
             price.unwrap_or(0.0)\n\
         }",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn names_the_import_each_one_needs() {
    let source = "context Shop {\n\
         fn a(name: String): Option<Int> = name.code_at(0)\n\
         fn b(code: Int): String = from_code(code)\n\
         fn c(number: Int): String = number.to_hex()\n\
         fn d(written: String): Option<Int> = from_hex(written)\n\
         }";

    assert_eq!(
        errors(source),
        [
            "`code_at` needs `import { code_at } from Std.Text`",
            "`from_code` needs `import { from_code } from Std.Text`",
            "`to_hex` needs `import { to_hex } from Std.Text`",
            "`from_hex` needs `import { from_hex } from Std.Text`",
        ]
    );
}

#[test]
fn checks_the_arguments_each_one_takes() {
    assert_error(
        &text("fn a(name: String): Option<Int> = name.code_at(\"0\")"),
        "expected `Int`, found `String`",
    );

    assert_error(
        &text("fn b(): String = from_code(\"A\")"),
        "expected `Int`, found `String`",
    );

    assert_error(
        &text("fn d(): Option<Int> = from_hex(255)"),
        "expected `String`, found `Int`",
    );

    assert_error(
        &text("fn d(): Option<Int> = from_hex(\"FF\", \"00\")"),
        "`from_hex` takes 1 arguments, found 2",
    );
}

/// A code known when the program is compiled is checked then.
#[test]
fn reports_a_known_character_code_past_255() {
    assert_error(
        &text("fn b(): String = from_code(256)"),
        "a character code is from 0 to 255, found 256",
    );

    assert_error(
        &text("fn b(): String = {\n let code = -1\n from_code(code)\n }"),
        "a character code is from 0 to 255, found -1",
    );

    assert_eq!(
        errors(&text("fn b(): String = from_code(0) + from_code(255)")),
        Vec::<String>::new()
    );
}

/// `to_decimal` takes its places from where its answer goes.
#[test]
fn needs_to_know_the_places_of_a_decimal() {
    assert_error(
        &text("fn e(written: String): Bool = written.to_decimal().is_some"),
        "`to_decimal` gives an `Option<Decimal<N>>`, so it needs to go where one with known places is expected, such as `let price: Option<Decimal<2>> = text.to_decimal()`",
    );

    assert_error(
        &text("fn e(written: String): Option<Int> = written.to_decimal()"),
        "`to_decimal` gives an `Option<Decimal<N>>`, so it needs to go where one with known places is expected, such as `let price: Option<Decimal<2>> = text.to_decimal()`",
    );
}
