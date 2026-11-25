//! Standard contexts written in Croft, such as `Std.Files`.

use super::{assert_error, errors};

const NOTES: &str = "context Notes {\n\
     publish { Keep }\n\
     use { Files, FileError, NotFound } from Std.Files\n\
     service Keep(): Result<Unit, FileError> {\n calls Files\n} = {\n\
         let data = files.read(\"notes.txt\")?\n\
         files.write(\"copy.txt\", data)\n\
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
            "`Files` needs `use { Files } from Std.Files`",
        ]
    );
}

#[test]
fn reports_a_name_a_standard_context_does_not_publish() {
    let source = NOTES.replace("NotFound }", "Missing }");
    assert_error(&source, "context `Std.Files` does not publish `Missing`");
}
