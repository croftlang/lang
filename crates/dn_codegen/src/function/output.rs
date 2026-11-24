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
        for parameter in &self.parameters {
            if !self.lines.iter().any(|line| mentions(line, parameter)) {
                out.push_str(&format!("    (void) {parameter};\n"));
            }
        }
        for line in &self.lines {
            out.push_str(line);
            out.push('\n');
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
