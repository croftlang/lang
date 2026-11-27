//! Tests for the formatter.

use std::path::{Path, PathBuf};

use crate::format_source;

/// `source` formatted, which must succeed.
fn formatted(source: &str) -> String {
    format_source(source).expect("the source formats")
}

#[test]
fn indents_by_the_braces_a_line_is_inside() {
    let source = "context Greeter {\nfn greet(): String = {\n  \"Hello\"\n      }\n}\n";

    let expected = "context Greeter {\n    fn greet(): String = {\n        \"Hello\"\n    }\n}\n";

    assert_eq!(formatted(source), expected);
}

#[test]
fn spaces_words_operators_and_punctuation() {
    let source = "context A {\nfn f( a :Int,b:Int ):Int={\nlet x=a+b*-1\nlet y = ! ( x>0 ) && x != 2\nlist . length ( )?\n}\n}\n";

    let expected = "context A {\n    fn f(a: Int, b: Int): Int = {\n        let x = a + b * -1\n        let y = !(x > 0) && x != 2\n        list.length()?\n    }\n}\n";

    assert_eq!(formatted(source), expected);
}

#[test]
fn writes_type_arguments_without_spaces() {
    let source = "context A {\nfn first < T > (items: List < List < T >>, price: Decimal < 2 >): Option < T > = {\nlet small = count < LIMIT\nitems\n}\n}\n";

    let expected = "context A {\n    fn first<T>(items: List<List<T>>, price: Decimal<2>): Option<T> = {\n        let small = count < LIMIT\n        items\n    }\n}\n";

    assert_eq!(formatted(source), expected);
}

#[test]
fn keeps_two_spaces_between_fields_written_without_commas() {
    let source =
        "context A {\nvalue Pair<A, B> { first: A second: B }\nvalue Point { x: Int, y: Int }\n}\n";

    let expected = "context A {\n    value Pair<A, B> { first: A  second: B }\n    value Point { x: Int, y: Int }\n}\n";

    assert_eq!(formatted(source), expected);
}

#[test]
fn keeps_every_comment_and_indents_it_with_the_code() {
    let source = "// The greeter.\ncontext Greeter {\n// Says hello.\n    fn greet(): String = {\n\"Hello\"   //   a greeting   \n// before the brace\n}\n}\n";

    let expected = "// The greeter.\ncontext Greeter {\n    // Says hello.\n    fn greet(): String = {\n        \"Hello\" //   a greeting\n        // before the brace\n    }\n}\n";

    assert_eq!(formatted(source), expected);
}

#[test]
fn cuts_blank_lines_to_one_and_drops_them_inside_brackets() {
    let source = "\n\ncontext A {\n\n\nconst X: Int = 1\n\n\n\nconst Y: Int = 2\n\n}\n\n\n";

    let expected = "context A {\n    const X: Int = 1\n\n    const Y: Int = 2\n}\n";

    assert_eq!(formatted(source), expected);
}

#[test]
fn indents_lines_that_carry_on_an_expression() {
    let source = "context A {\nfn f(): Int = {\nitems\n.map(x -> x)\n.length()\nlet total = 1 +\n2\ntotal\n}\n}\n";

    let expected = "context A {\n    fn f(): Int = {\n        items\n            .map(x -> x)\n            .length()\n        let total = 1 +\n            2\n        total\n    }\n}\n";

    assert_eq!(formatted(source), expected);
}

#[test]
fn keeps_one_space_after_contextual_words() {
    let source = "context Shop {\n    identity ShelfId for Shelf: Int where Range(1, 99)\n    aggregate Shelf { id: ShelfId }\n}\n";

    assert_eq!(formatted(source), source);
}

#[test]
fn keeps_numbers_and_strings_as_written() {
    let source = "context A {\n    const MASK: Int = 0b1010_0001\n    const BIG: Int = 999_999\n    const TEXT: String = \"tab\\tquote\\\"\"\n}\n";

    assert_eq!(formatted(source), source);
}

#[test]
fn keeps_files_without_braces_at_the_left() {
    let source = "context Shipping\naggregate Cargo\n\ntracking_id:TrackingId\n\nevent Routed { itinerary: Itinerary }\n";

    let expected = "context Shipping\naggregate Cargo\n\ntracking_id: TrackingId\n\nevent Routed { itinerary: Itinerary }\n";

    assert_eq!(formatted(source), expected);
}

#[test]
fn refuses_a_file_that_does_not_parse() {
    let error = format_source("context A {\n").unwrap_err();

    assert!(error.message.contains("expected"), "{}", error.message);
}

#[test]
fn formats_an_empty_file_as_empty() {
    assert_eq!(formatted(""), "");
    assert_eq!(formatted("\n\n"), "");
}

/// Every Croft file in the repository, from the examples to the standard library.
fn repository_sources() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();

    for folder in [
        "examples",
        "tests/fixtures",
        "crates/croft_std/std",
        "crates/croft_codegen/tests",
    ] {
        collect_sources(&root.join(folder), &mut found);
    }

    found
}

/// Adds every source file in `folder` and its subfolders to `found`.
fn collect_sources(folder: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            collect_sources(&path, found);
            continue;
        }

        if croft_syntax::is_source_file(&path) {
            found.push(path);
        }
    }
}

#[test]
fn formats_every_file_in_the_repository_and_formatting_again_changes_nothing() {
    let sources = repository_sources();

    assert!(sources.len() > 20, "found {} files", sources.len());

    for path in sources {
        let source = std::fs::read_to_string(&path).expect("the file reads");

        // Some test programs are meant not to parse.
        if croft_syntax::parse(&source, croft_syntax::SourceId(0)).is_err() {
            continue;
        }

        let once = format_source(&source)
            .unwrap_or_else(|error| panic!("{}: {}", path.display(), error.message));

        let twice = format_source(&once)
            .unwrap_or_else(|error| panic!("{} again: {}", path.display(), error.message));

        assert_eq!(
            once,
            twice,
            "{} changes when formatted again",
            path.display()
        );
    }
}

#[test]
fn leaves_the_logistics_fixture_and_the_standard_library_as_written() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut sources = Vec::new();

    collect_sources(&root.join("tests/fixtures/logistics"), &mut sources);
    collect_sources(&root.join("crates/croft_std/std"), &mut sources);

    assert!(sources.len() > 10, "found {} files", sources.len());

    for path in sources {
        let source = std::fs::read_to_string(&path).expect("the file reads");
        assert_eq!(formatted(&source), source, "{} changed", path.display());
    }
}

#[test]
fn puts_back_the_indentation_of_every_file_in_the_repository() {
    for path in repository_sources() {
        let source = std::fs::read_to_string(&path).expect("the file reads");

        if croft_syntax::parse(&source, croft_syntax::SourceId(0)).is_err() {
            continue;
        }

        let flattened: String = source
            .lines()
            .map(|line| format!("{}\n", line.trim_start()))
            .collect();

        assert_eq!(
            formatted(&flattened),
            formatted(&source),
            "{}",
            path.display()
        );
    }
}
