//! Standard contexts written in Croft, such as `Std.Files`.

use super::{assert_error, errors};

const NOTES: &str = "context Notes {\n\
     publish { Keep }\n\
     use { Files, FileError, NotFound } from Std.Files\n\
     service Keep(): Result<Unit, FileError> {\n calls { Files }\n} = {\n\
         let data = Files.read(\"notes.txt\", 4096)?\n\
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

/// `Files.read` says the most it reads, which sizes its result, so a
/// number not known when the program is compiled is an error, as is one
/// outside `Range(1, 65535)`. Leaving it out names it.
#[test]
fn reads_a_file_of_at_most_a_known_size() {
    let read = |given: &str| NOTES.replace("let data = Files.read(\"notes.txt\", 4096)?", given);

    let limit = "let limit = 512\nlet data = Files.read(\"notes.txt\", limit * 2)?";
    assert_eq!(errors(&read(limit)), Vec::<String>::new());

    assert_error(
        &read("let data = Files.read(\"notes.txt\")?"),
        "`read` needs a value for `most`",
    );

    assert_error(
        &read("let data = Files.read(\"notes.txt\", 70000)?"),
        "`Files.read` is given `70000` for `most`, which breaks `Range(1, 65535)`",
    );

    let measured = "let first = Files.read(\"notes.txt\", 16)?\n\
                    let data = Files.read(\"notes.txt\", first.length)?";

    assert_error(
        &read(measured),
        "`most` must be known when the program is compiled, since it says how much room \
         `Files.read` keeps for its result: use a number or a constant",
    );
}

/// `Files.read_part` says how many bytes it reads, which sizes its result,
/// so a length not known when the program is compiled is an error, as is
/// one outside `Range(1, 65535)` or an offset below 0. The offset may be
/// known only when it runs.
#[test]
fn reads_part_of_a_file_of_a_known_length() {
    let read = |given: &str| NOTES.replace("let data = Files.read(\"notes.txt\", 4096)?", given);

    let offset = "let size = Files.size(\"notes.txt\")?\n\
                  let data = Files.read_part(\"notes.txt\", size - 16, 16)?";
    assert_eq!(errors(&read(offset)), Vec::<String>::new());

    assert_error(
        &read("let data = Files.read_part(\"notes.txt\", 0, 0)?"),
        "`Files.read_part` is given `0` for `length`, which breaks `Range(1, 65535)`",
    );

    assert_error(
        &read("let data = Files.read_part(\"notes.txt\", -1, 16)?"),
        "`Files.read_part` is given `-1` for `offset`, which breaks `Range(0, 2147483647)`",
    );

    let measured = "let size = Files.size(\"notes.txt\")?\n\
                    let data = Files.read_part(\"notes.txt\", 0, size)?";

    assert_error(
        &read(measured),
        "`length` must be known when the program is compiled, since it says how much room \
         `Files.read_part` keeps for its result: use a number or a constant",
    );
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

/// A calendar that makes `date` from Std.Time.
fn calendar(date: &str) -> String {
    format!(
        "context Calendar {{\n\
         publish {{ due }}\n\
         use {{ Date }} from Std.Time\n\
         fn due(): Date = {date}\n\
         }}\n\
         domain Program {{\n contexts {{ Calendar }}\n use {{ due }} from Calendar\n}} = {{\n due()\n Unit\n}}"
    )
}

/// A `Date` is a day the calendar has: 30 February is an error, and 29
/// February only in a leap year.
#[test]
fn reports_a_known_date_that_does_not_exist() {
    let broken = |date: &str| format!("`{date}` breaks `Date`'s invariant `DayInMonth`");

    for date in [
        "Date { year: 2025, month: 2, day: 29 }",
        "Date { year: 1900, month: 2, day: 29 }",
        "Date { year: 2024, month: 2, day: 30 }",
        "Date { year: 2026, month: 4, day: 31 }",
        "Date { year: 2026, month: 11, day: 31 }",
    ] {
        assert_error(&calendar(date), &broken(date));
    }

    for date in [
        "Date { year: 2024, month: 2, day: 29 }",
        "Date { year: 2000, month: 2, day: 29 }",
        "Date { year: 2026, month: 2, day: 28 }",
        "Date { year: 2026, month: 4, day: 30 }",
        "Date { year: 2026, month: 12, day: 31 }",
        "Date { year: 2026, month: 7, day: 31 }",
        "Date { year: 2026, month: 8, day: 31 }",
    ] {
        assert_eq!(errors(&calendar(date)), Vec::<String>::new(), "{date}");
    }
}
