//! Writing lines with indentation, and the finished function.

use super::Writer;

impl<'g, 'a> Writer<'g, 'a> {
    pub fn line(&mut self, text: impl AsRef<str>) {
        let text = tidy_condition(text.as_ref());
        self.lines
            .push(format!("{}{text}", "    ".repeat(self.depth)));
    }

    /// Writes `header {` and indents what follows until `close`.
    pub fn open(&mut self, header: impl AsRef<str>) {
        self.line(format!("{} {{", header.as_ref()));
        self.depth += 1;
    }

    /// A preprocessor line, such as `#ifdef DN_DEBUG`, at the start of the line.
    pub fn directive(&mut self, text: &str) {
        self.lines.push(text.to_string());
    }

    pub fn close(&mut self) {
        self.depth -= 1;
        self.line("}");
    }

    /// Lines written since `mark`, taken out so they can be placed elsewhere.
    pub fn mark(&self) -> usize {
        self.lines.len()
    }

    pub fn take_since(&mut self, mark: usize) -> Vec<String> {
        self.lines.split_off(mark)
    }

    pub fn put_back(&mut self, lines: Vec<String>) {
        for line in lines {
            self.lines.push(format!("    {line}"));
        }
    }

    /// The finished function: its signature, declarations, then statements.
    pub fn finish(self, signature: &str) -> String {
        let mut out = format!("{signature}\n{{\n");
        let used = |declaration: &&String| {
            let name = declaration
                .trim_end_matches(';')
                .rsplit(' ')
                .next()
                .unwrap_or_default();
            self.lines.iter().any(|line| mentions(line, name))
        };
        let declarations: Vec<&String> = self.declarations.iter().filter(used).collect();
        for declaration in &declarations {
            out.push_str(declaration);
            out.push('\n');
        }
        if !declarations.is_empty() && !self.lines.is_empty() {
            out.push('\n');
        }
        // A parameter only used in debug-build lines is still unused otherwise.
        let release: Vec<&String> = release_lines(&self.lines);
        for parameter in &self.parameters {
            if !release.iter().any(|line| mentions(line, parameter)) {
                out.push_str(&format!("    (void) {parameter};\n"));
            }
        }
        for line in &self.lines {
            out.push_str(line);
            out.push('\n');
        }
        // A variable a release build only writes to, such as a value built
        // just to be checked in debug builds, is marked as used.
        for declaration in &declarations {
            let name = declaration
                .trim_end_matches(';')
                .rsplit(' ')
                .next()
                .unwrap_or_default();
            if release
                .iter()
                .all(|line| !mentions(line, name) || is_write_to(line, name))
            {
                out.push_str(&format!("    (void) {name};\n"));
            }
        }
        out.push_str("}\n\n");
        out
    }
}
/// Whether `line` uses the C name `name`, not just as part of a longer name.
pub(super) fn mentions(line: &str, name: &str) -> bool {
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_';
    line.match_indices(name).any(|(start, _)| {
        let before = line[..start].chars().next_back();
        let after = line[start + name.len()..].chars().next();
        !before.is_some_and(is_name) && !after.is_some_and(is_name)
    })
}

/// Turns `if ((a == b))` into `if (a == b)`, which compilers prefer.
fn tidy_condition(text: &str) -> String {
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

/// The lines a build without `DN_DEBUG` compiles.
fn release_lines(lines: &[String]) -> Vec<&String> {
    let mut in_debug = false;
    let mut release = Vec::new();
    for line in lines {
        match line.trim() {
            "#ifdef DN_DEBUG" => in_debug = true,
            "#endif" if in_debug => in_debug = false,
            _ if !in_debug => release.push(line),
            _ => {}
        }
    }
    release
}

/// Whether `line` only writes to `name`: `name = ...` or `name.field = ...`.
fn is_write_to(line: &str, name: &str) -> bool {
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
