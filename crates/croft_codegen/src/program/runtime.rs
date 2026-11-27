//! The runtime a program is given: only the functions it calls. cc65 and
//! z88dk link a whole C file at once, so a function left in but never
//! called still costs memory on an 8-bit machine.

use crate::function::mentions;

/// One function of the runtime, with the comment above it.
struct Piece<'r> {
    name: &'r str,
    text: String,
}

/// The runtime source with every function that `program` doesn't reach,
/// directly or through other runtime functions, left out.
pub fn trim_runtime(runtime: &str, program: &str) -> String {
    let (preamble, pieces) = split_functions(runtime);
    let mut kept = vec![false; pieces.len()];
    let mut reached = vec![program.to_string()];

    loop {
        let mut changed = false;

        for (index, piece) in pieces.iter().enumerate() {
            if !kept[index] && reached.iter().any(|text| mentions(text, piece.name)) {
                kept[index] = true;
                reached.push(piece.text.clone());
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    let mut source = preamble;

    for (piece, keep) in pieces.iter().zip(kept) {
        if keep {
            source.push_str(&piece.text);
        }
    }

    source
}

/// `program` and the body of each macro in `header` that it uses, such as
/// `CROFT_VIEW`, so the functions those call are kept too.
pub fn with_macros(header: &str, program: &str) -> String {
    let mut reached = program.to_string();

    for line in header.lines() {
        let Some(definition) = line.strip_prefix("#define ") else {
            continue;
        };

        let name_end = definition.find(['(', ' ']).unwrap_or(definition.len());

        if mentions(program, &definition[..name_end]) {
            reached.push('\n');
            reached.push_str(&definition[name_end..]);
        }
    }

    reached
}

/// Splits C source into what comes before the first function, and each
/// function with the comments and blank lines just above it. A function
/// starts on a line that isn't indented and opens with a name and `(`, and
/// ends at a line that is only `}`.
fn split_functions(source: &str) -> (String, Vec<Piece<'_>>) {
    let mut preamble = String::new();
    let mut pieces = Vec::new();
    let mut pending = String::new();
    let mut current: Option<(&str, String)> = None;

    for line in source.lines() {
        if let Some((name, text)) = current.as_mut() {
            text.push_str(line);
            text.push('\n');

            if line == "}" {
                pieces.push(Piece {
                    name,
                    text: std::mem::take(text),
                });

                current = None;
            }

            continue;
        }

        match function_name(line) {
            Some(name) => current = Some((name, std::mem::take(&mut pending) + line + "\n")),
            None if pieces.is_empty() && !is_comment_or_blank(line) => {
                preamble.push_str(&std::mem::take(&mut pending));
                preamble.push_str(line);
                preamble.push('\n');
            }
            None => {
                pending.push_str(line);
                pending.push('\n');
            }
        }
    }

    if pieces.is_empty() {
        preamble.push_str(&pending);
    }

    (preamble, pieces)
}

/// The name a line starts a function definition with, if it does.
fn function_name(line: &str) -> Option<&str> {
    if line.starts_with([' ', '\t', '#', '/', '*', '}']) || line.ends_with(';') {
        return None;
    }

    let before = &line[..line.find('(')?];
    let name = before.rsplit([' ', '*']).next()?;

    (!name.is_empty()).then_some(name)
}

/// Whether a line is blank or part of a `/* */` comment.
fn is_comment_or_blank(line: &str) -> bool {
    let line = line.trim_start();
    line.is_empty() || line.starts_with("/*") || line.starts_with('*')
}

/// Tests for leaving out runtime functions the program never calls.
#[cfg(test)]
mod tests {
    use super::trim_runtime;

    /// A small runtime: one function used directly, one through it, and one
    /// never called.
    const RUNTIME: &str = "#include \"croft.h\"\n\n/* Used by both. */\nstatic int helper(void)\n{\n    return 1;\n}\n\nint croft_used(void)\n{\n    return helper();\n}\n\n/* Never called. */\nint croft_unused(void)\n{\n    return 2;\n}\n";

    /// The runtime keeps what the program calls, directly or not, and drops
    /// the rest with its comment.
    #[test]
    fn keeps_only_what_the_program_reaches() {
        let trimmed = trim_runtime(RUNTIME, "x = croft_used();");
        assert!(trimmed.contains("#include \"croft.h\""));
        assert!(trimmed.contains("int croft_used(void)"));
        assert!(trimmed.contains("static int helper(void)"));
        assert!(trimmed.contains("/* Used by both. */"));
        assert!(!trimmed.contains("croft_unused"));
        assert!(!trimmed.contains("Never called"));
    }
}
