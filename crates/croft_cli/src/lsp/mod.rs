//! `croft lsp`: a language server, so editors can show errors as you type, what
//! a name is on hover, where it is declared and what can be typed next, and
//! can format a file.
//!
//! It speaks the language server protocol over stdin and stdout. A change
//! to a file checks its project again, with the files open in the editor
//! read as they are there, saved or not. Changes made in quick succession,
//! as when typing, are checked once, when they pause or when the editor
//! asks something.

mod completion;
mod lookup;
mod position;
mod session;
mod standard;
#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossbeam_channel::RecvTimeoutError;

use lsp_server::{Connection, ExtractError, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _, PublishDiagnostics,
};
use lsp_types::request::{Completion, Formatting, GotoDefinition, HoverRequest, Request as _};
use lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, Diagnostic, DiagnosticSeverity,
    DocumentFormattingParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents,
    HoverParams, HoverProviderCapability, InitializeParams, Location, MarkupContent, MarkupKind,
    OneOf, PublishDiagnosticsParams, ServerCapabilities, TextDocumentPositionParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Uri,
};

use self::lookup::Found;
use self::position::{
    from_position, name_range, path_to_uri, uri_to_path, whole_range, word_range,
};
use self::session::{ProjectState, Session};
use croft_syntax::SourceId;

/// Runs the language server until the editor shuts it down.
pub fn run() -> Result<(), String> {
    let (connection, io_threads) = Connection::stdio();

    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_string()]),
            ..CompletionOptions::default()
        }),
        ..ServerCapabilities::default()
    };

    let capabilities = serde_json::to_value(capabilities).map_err(|e| e.to_string())?;

    let initialize = connection
        .initialize(capabilities)
        .map_err(|error| error.to_string())?;

    let params: InitializeParams =
        serde_json::from_value(initialize).map_err(|error| error.to_string())?;

    let mut server = Server {
        connection: &connection,
        session: Session::new(workspace_root(&params)),
        changed: HashSet::new(),
    };

    server.serve()?;
    // The thread reading stdin only ends when the editor closes it, which
    // some editors do only after the server exits, so it is not waited for.
    drop(io_threads);

    Ok(())
}

/// The first workspace folder the editor opened, if any.
fn workspace_root(params: &InitializeParams) -> Option<std::path::PathBuf> {
    let folder = params.workspace_folders.as_ref()?.first()?;
    uri_to_path(&folder.uri)
}

/// How long typing must pause before the files changed are checked.
const PAUSE: Duration = Duration::from_millis(150);

/// The connection to the editor, and what is known about its files.
struct Server<'a> {
    connection: &'a Connection,
    session: Session,
    /// Files changed since their projects were last checked.
    changed: HashSet<PathBuf>,
}

impl Server<'_> {
    /// Handles messages until the editor shuts the server down, checking
    /// changed files whenever typing pauses.
    fn serve(&mut self) -> Result<(), String> {
        loop {
            let message = if self.changed.is_empty() {
                match self.connection.receiver.recv() {
                    Ok(message) => message,
                    Err(_) => return Ok(()),
                }
            } else {
                match self.connection.receiver.recv_timeout(PAUSE) {
                    Ok(message) => message,
                    Err(RecvTimeoutError::Timeout) => {
                        self.check_changed();
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => return Ok(()),
                }
            };

            match message {
                Message::Request(request) => {
                    let shutdown = self.connection.handle_shutdown(&request);

                    if shutdown.map_err(|error| error.to_string())? {
                        return Ok(());
                    }

                    // An answer is about the text as it is now.
                    self.check_changed();
                    self.request(request);
                }
                Message::Notification(notification) => self.notification(notification),
                Message::Response(_) => {}
            }
        }
    }

    /// Checks each project with a changed file, once.
    fn check_changed(&mut self) {
        let mut checked = HashSet::new();

        for path in std::mem::take(&mut self.changed) {
            if checked.insert(self.session.project_key(&path)) {
                self.refresh(&path);
            }
        }
    }

    /// Answers one request from the editor.
    fn request(&mut self, request: Request) {
        let id = request.id.clone();

        let result = match request.method.as_str() {
            HoverRequest::METHOD => params::<HoverParams>(request)
                .map(|p| serde_json::to_value(self.hover(&p.text_document_position_params))),
            GotoDefinition::METHOD => params::<GotoDefinitionParams>(request)
                .map(|p| serde_json::to_value(self.definition(&p.text_document_position_params))),
            Completion::METHOD => params::<CompletionParams>(request)
                .map(|p| serde_json::to_value(self.complete(&p.text_document_position))),
            Formatting::METHOD => params::<DocumentFormattingParams>(request)
                .map(|p| serde_json::to_value(self.format(&p.text_document.uri))),
            _ => {
                self.reply(Response::new_err(id, -32601, "unknown request".into()));
                return;
            }
        };

        let response = match result {
            Ok(Ok(value)) => Response::new_ok(id, value),
            Ok(Err(error)) => Response::new_err(id, -32603, error.to_string()),
            Err(message) => Response::new_err(id, -32602, message),
        };

        self.reply(response);
    }

    /// Sends a response to the editor.
    fn reply(&self, response: Response) {
        let _ = self.connection.sender.send(Message::Response(response));
    }

    /// Records a file opened, changed, saved or closed in the editor.
    fn notification(&mut self, notification: Notification) {
        let method = notification.method.clone();

        let changed = match method.as_str() {
            DidOpenTextDocument::METHOD => notification_params::<DidOpenTextDocument>(notification)
                .and_then(|p| {
                    let path = uri_to_path(&p.text_document.uri)?;
                    self.session.open(path.clone(), p.text_document.text);
                    Some(path)
                }),
            DidChangeTextDocument::METHOD => {
                notification_params::<DidChangeTextDocument>(notification).and_then(|p| {
                    let path = uri_to_path(&p.text_document.uri)?;
                    let text = p.content_changes.into_iter().last()?.text;
                    self.session.open(path.clone(), text);
                    Some(path)
                })
            }
            DidSaveTextDocument::METHOD => notification_params::<DidSaveTextDocument>(notification)
                .and_then(|p| uri_to_path(&p.text_document.uri)),
            DidCloseTextDocument::METHOD => {
                notification_params::<DidCloseTextDocument>(notification).and_then(|p| {
                    let path = uri_to_path(&p.text_document.uri)?;
                    self.session.close(&path);
                    Some(path)
                })
            }
            _ => None,
        };

        // The standard library's files are only there to be read.
        if let Some(path) = changed.filter(|path| standard::source_at(path).is_none()) {
            self.changed.insert(path);
        }
    }

    /// Checks the project a file belongs to again, and sends its errors.
    fn refresh(&mut self, path: &Path) {
        let Ok(key) = self.session.refresh(path) else {
            return;
        };

        let state = &self.session.projects[&key];

        for file in &state.files {
            let Some(uri) = path_to_uri(&file.path) else {
                continue;
            };

            let text = self.session.current_text(file);

            let diagnostics = state
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.span.source == file.id)
                .map(|diagnostic| Diagnostic {
                    range: word_range(text, diagnostic.span),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("croft".to_string()),
                    message: diagnostic.message.clone(),
                    ..Diagnostic::default()
                })
                .collect();

            let params = PublishDiagnosticsParams::new(uri, diagnostics, None);
            let notification = Notification::new(PublishDiagnostics::METHOD.to_string(), params);

            let _ = self
                .connection
                .sender
                .send(Message::Notification(notification));
        }
    }

    /// The project a file is checked in, the file's source id, and its text
    /// as checked. A standard library file is read in any project that uses it.
    fn file<'s>(&'s self, path: &Path) -> Option<(&'s ProjectState, SourceId, &'s str)> {
        if let Some((id, text)) = standard::source_at(path) {
            let mut projects = self.session.projects.values();

            let state = projects.find(|state| {
                (state.analysis.symbols.values())
                    .any(|table| table.symbols.values().any(|s| s.span.source == id))
            })?;

            return Some((state, id, text));
        }

        let state = self.session.project_for(path)?;
        let file = state.file_at(path)?;

        Some((state, file.id, &file.text))
    }

    /// The project, the name under the cursor and the file's text.
    fn at<'s>(
        &'s self,
        at: &TextDocumentPositionParams,
    ) -> Option<(&'s ProjectState, Found<'s>, &'s str)> {
        let path = uri_to_path(&at.text_document.uri)?;
        let (state, id, text) = self.file(&path)?;
        let (line, column) = from_position(text, at.position);
        let found = lookup::find_at(state, id, line, column)?;
        Some((state, found, text))
    }

    /// What the name under the cursor is, in Markdown.
    fn hover(&self, at: &TextDocumentPositionParams) -> Option<Hover> {
        let (state, found, text) = self.at(at)?;
        let markdown = lookup::hover(state, &found)?;
        let (span, length) = found.written();

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown,
            }),
            range: Some(name_range(text, span, length)),
        })
    }

    /// Where the name under the cursor is declared.
    fn definition(&self, at: &TextDocumentPositionParams) -> Option<GotoDefinitionResponse> {
        let (state, found, _) = self.at(at)?;
        let (span, length) = lookup::definition(state, &found)?;

        let (path, text) = match state.file(span.source) {
            Some(file) => (file.path.clone(), file.text.as_str()),
            // The standard library, written out to be opened.
            None => (standard::path_of(span.source)?, state.text(span.source)?),
        };

        let uri: Uri = path_to_uri(&path)?;
        let range = name_range(text, span, length);

        Some(GotoDefinitionResponse::Scalar(Location::new(uri, range)))
    }

    /// What can be typed at the cursor.
    fn complete(&self, at: &TextDocumentPositionParams) -> Option<CompletionResponse> {
        let path = uri_to_path(&at.text_document.uri)?;
        let (state, id, checked) = self.file(&path)?;
        // What is typed now, which may not have parsed.
        let text = self.session.open_text(&path).unwrap_or(checked);
        let (line, column) = from_position(text, at.position);
        let items = completion::complete(state, id, text, line, column);
        Some(CompletionResponse::Array(items))
    }

    /// The file laid out in the Croft style, as one edit replacing all of
    /// it, or no edits when it already is. Nothing when it does not parse,
    /// whose errors are already shown.
    fn format(&self, uri: &Uri) -> Option<Vec<TextEdit>> {
        let path = uri_to_path(uri)?;

        // The standard library's files are only there to be read.
        if standard::source_at(&path).is_some() {
            return None;
        }

        let saved;

        let text = match self.session.open_text(&path) {
            Some(text) => text,
            None => {
                saved = std::fs::read_to_string(&path).ok()?;
                saved.as_str()
            }
        };

        let formatted = croft_format::format_source(text).ok()?;

        if formatted == text {
            return Some(Vec::new());
        }

        Some(vec![TextEdit::new(whole_range(text), formatted)])
    }
}

/// The parameters of a request, or why they could not be read.
fn params<P: serde::de::DeserializeOwned>(request: Request) -> Result<P, String> {
    let method = request.method.clone();

    let (_, params) = request
        .extract::<P>(&method)
        .map_err(|error: ExtractError<Request>| format!("{error:?}"))?;

    Ok(params)
}

/// The parameters of a notification, if they can be read.
fn notification_params<N: lsp_types::notification::Notification>(
    notification: Notification,
) -> Option<N::Params> {
    notification.extract(N::METHOD).ok()
}
