//! Standard contexts written in Croft, such as `Std.Files`.

use super::{assert_error, errors};

const NOTES: &str = "context Notes {\n\
     publish { Keep }\n\
     use { Files, FileError, NotFound } from Std.Files\n\
     service Keep(): Result<Unit, FileError> {\n calls { Files }\n} = {\n\
         let data = Files.read(\"notes.txt\")?\n\
         Files.write(\"copy.txt\", data)\n\
     }\n\
     }\n\
     domain Program {\n contexts { Notes }\n use { Keep } from Notes\n} = Keep()?";

#[test]
fn joins_a_standard_context_when_it_is_imported() {
    assert_eq!(errors(NOTES), Vec::<String>::new());
}

#[test]
fn names_the_import_a_standard_name_needs() {
    let source = NOTES.replace("use { Files, FileError, NotFound } from Std.Files", "");

    assert_eq!(
        errors(&source),
        [
            "`FileError` needs `use { FileError } from Std.Files`",
            "`Files` needs `calls { Files } from Std.Files`",
        ]
    );
}

#[test]
fn reports_a_name_a_standard_context_does_not_publish() {
    let source = NOTES.replace("NotFound }", "Missing }");
    assert_error(&source, "context `Std.Files` does not publish `Missing`");
}

/// A game rolling a die through Std.Random, with `roll` in its body.
fn dice(roll: &str) -> String {
    format!(
        "context Dice {{\n\
         publish {{ Roll }}\n\
         use {{ Random }} from Std.Random\n\
         service Roll(sides: Int): Int {{\n calls {{ Random }}\n}} = {{\n{roll}\n}}\n\
         }}\n\
         domain Program {{\n contexts {{ Dice }}\n use {{ Roll }} from Dice\n}} = {{\n Roll(6)\n Unit\n}}"
    )
}

#[test]
fn joins_std_random_when_it_is_imported() {
    assert_eq!(
        errors(&dice("Random.between(1, sides)")),
        Vec::<String>::new()
    );
    assert_eq!(errors(&dice("Random.between(4, 4)")), Vec::<String>::new());
}

/// `low <= high` is the caller's to keep, so known numbers that break it
/// are an error, and unknown ones are left to the program.
#[test]
fn reports_a_known_low_above_high() {
    assert_error(
        &dice("Random.between(6, 1)"),
        "`Random.between(6, 1)` breaks `low <= high`",
    );

    assert_error(
        &dice("let low = 7\nRandom.between(low, 6)"),
        "`Random.between(7, 6)` breaks `low <= high`",
    );

    assert_eq!(
        errors(&dice("Random.between(sides, 1)")),
        Vec::<String>::new()
    );
}
