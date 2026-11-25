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

fn is_comment_or_blank(line: &str) -> bool {
    let line = line.trim_start();
    line.is_empty() || line.starts_with("/*") || line.starts_with('*')
}

#[cfg(test)]
mod tests {
    use super::trim_runtime;

    const RUNTIME: &str = "#include \"b.h\"\n\n/* Used by both. */\nstatic int helper(void)\n{\n    return 1;\n}\n\nint b_used(void)\n{\n    return helper();\n}\n\n/* Never called. */\nint b_unused(void)\n{\n    return 2;\n}\n";

    #[test]
    fn keeps_only_what_the_program_reaches() {
        let trimmed = trim_runtime(RUNTIME, "x = b_used();");
        assert!(trimmed.contains("#include \"b.h\""));
        assert!(trimmed.contains("int b_used(void)"));
        assert!(trimmed.contains("static int helper(void)"));
        assert!(trimmed.contains("/* Used by both. */"));
        assert!(!trimmed.contains("b_unused"));
        assert!(!trimmed.contains("Never called"));
    }
}
