//! Printing diagnostics with the offending source line.

use croft_syntax::Diagnostic;

use crate::project::Project;

/// Prints each diagnostic, then how many there were.
pub fn print_diagnostics(project: &Project, diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        eprintln!("{}", format_diagnostic(project, diagnostic));
    }

    let count = diagnostics.len();
    eprintln!("\n{count} error{} found", if count == 1 { "" } else { "s" });
}

/// A diagnostic as `file:line:column: error: message`, followed by its
/// source line with a `^` under the column.
fn format_diagnostic(project: &Project, diagnostic: &Diagnostic) -> String {
    let span = diagnostic.span;

    let Some(file) = project.files.get(span.source.0 as usize) else {
        return format!("error: {}", diagnostic.message);
    };

    let mut text = format!(
        "{}:{}:{}: error: {}",
        file.path.display(),
        span.line,
        span.column,
        diagnostic.message
    );

    if let Some(line) = file.text.lines().nth(span.line.saturating_sub(1) as usize) {
        let gutter = span.line.to_string();
        let padding = " ".repeat(gutter.len());

        let caret_offset: String = line
            .chars()
            .take(span.column.saturating_sub(1) as usize)
            .map(|c| if c == '\t' { '\t' } else { ' ' })
            .collect();

        text.push_str(&format!(
            "\n{padding} |\n{gutter} | {line}\n{padding} | {caret_offset}^"
        ));
    }

    text
}
