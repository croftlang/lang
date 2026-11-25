//! Starts `b lsp`, the language server in the `b` command, for `.b` files.

use zed_extension_api::{self as zed, LanguageServerId, Result};

struct BExtension;

impl zed::Extension for BExtension {
    fn new() -> Self {
        BExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let command = worktree.which("b").ok_or_else(|| {
            "`b` is not on your PATH. Install it with `cargo install --path crates/b_cli`."
                .to_string()
        })?;
        Ok(zed::Command {
            command,
            args: vec!["lsp".to_string()],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(BExtension);
