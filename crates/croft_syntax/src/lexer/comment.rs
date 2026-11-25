//! Whitespace and `//` comments. Comment lines directly above a token are its doc.

use super::Lexer;

impl<'a> Lexer<'a> {
    /// Skips whitespace and `//` comments. Returns the comment lines that sit
    /// directly above the next token, with no blank line between.
    pub(crate) fn skip_whitespace_and_comments(&mut self) -> Vec<String> {
        let mut doc = Vec::new();
        let mut last_comment_line = 0;

        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_second() == Some('/') => {
                    let line = self.current_span().line;

                    if !doc.is_empty() && line != last_comment_line + 1 {
                        doc.clear();
                    }

                    doc.push(self.read_comment());
                    last_comment_line = line;
                }
                _ => break,
            }
        }

        let next_line = self.current_span().line;

        if !doc.is_empty() && next_line != last_comment_line + 1 {
            doc.clear();
        }

        doc
    }

    fn read_comment(&mut self) -> String {
        self.advance();
        self.advance();

        let mut text = String::new();

        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }

            text.push(c);
            self.advance();
        }

        text.trim().to_string()
    }
}
