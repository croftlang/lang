//! Starts `croft lsp`, the language server in the `croft` command, for `.croft` and `.cro` files.

use zed_extension_api::{self as zed, LanguageServerId, Result};

struct CroftExtension;

impl zed::Extension for CroftExtension {
    fn new() -> Self {
        CroftExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let command = worktree.which("croft").ok_or_else(|| {
            "`croft` is not on your PATH. Install it with `cargo install --path crates/croft_cli`."
                .to_string()
        })?;
        Ok(zed::Command {
            command,
            args: vec!["lsp".to_string()],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(CroftExtension);
