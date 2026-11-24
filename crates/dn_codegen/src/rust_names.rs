const RUST_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
    "virtual", "where", "while", "yield",
];

/// Escapes an identifier that clashes with a Rust keyword, e.g. `type` -> `r#type`.
pub fn rust_identifier(name: &str) -> String {
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_keywords() {
        assert_eq!(rust_identifier("type"), "r#type");
        assert_eq!(rust_identifier("amount"), "amount");
    }
}
