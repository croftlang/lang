//! Tidying the finished C: what a build without checks uses, and conditions
//! written with one pair of brackets.

/// Whether `line` uses the C name `name`, not just as part of a longer name.
pub(crate) fn mentions(line: &str, name: &str) -> bool {
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';

    line.match_indices(name).any(|(start, _)| {
        let before = line[..start].chars().next_back();
        let after = line[start + name.len()..].chars().next();
        !before.is_some_and(is_name) && !after.is_some_and(is_name)
    })
}

/// Turns `if ((a == b))` into `if (a == b)`, which compilers prefer.
pub(super) fn tidy_condition(text: &str) -> String {
    let Some(start) = text.find("if ((") else {
        return text.to_string();
    };

    let inner = start + 4;
    let mut depth = 0;

    for (offset, c) in text[inner..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;

                if depth == 0 {
                    let close = inner + offset;

                    if text[close + 1..].starts_with(')') {
                        return format!(
                            "{}{}{}",
                            &text[..inner],
                            &text[inner + 1..close],
                            &text[close + 1..]
                        );
                    }

                    return text.to_string();
                }
            }
            _ => {}
        }
    }

    text.to_string()
}

/// The lines a build without `CROFT_CHECKS` compiles.
pub(super) fn unchecked_lines(lines: &[String]) -> Vec<&String> {
    let mut in_checks = false;
    let mut unchecked = Vec::new();

    for line in lines {
        match line.trim() {
            "#ifdef CROFT_CHECKS" => in_checks = true,
            "#endif" if in_checks => in_checks = false,
            _ if !in_checks => unchecked.push(line),
            _ => {}
        }
    }

    unchecked
}

/// Whether `line` only writes to `name`: `name = ...` or `name.field = ...`.
pub(super) fn is_write_to(line: &str, name: &str) -> bool {
    let line = line.trim_start();

    let Some(rest) = line.strip_prefix(name) else {
        return false;
    };

    let target_end = rest.find(" = ").unwrap_or(usize::MAX);
    let path = &rest[..target_end.min(rest.len())];

    target_end != usize::MAX
        && path
            .chars()
            .all(|c| c == '.' || c == '_' || c.is_ascii_alphanumeric())
        && !rest[target_end + 3..].contains(name)
}
