//! `b lsp`: a language server, so editors can show errors as you type, what
//! a name is on hover, and where it is declared.
//!
//! It speaks the language server protocol over stdin and stdout. Each change
//! to a file checks its project again, with the files open in the editor
//! read as they are there, saved or not.

mod lookup;
mod position;
mod session;
#[cfg(test)]
mod tests;

use std::path::Path;

use lsp_server::{Connection, ExtractError, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _, PublishDiagnostics,
};
use lsp_types::request::{GotoDefinition, HoverRequest, Request as _};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, HoverProviderCapability, InitializeParams, Location, MarkupContent,
    MarkupKind, OneOf, PublishDiagnosticsParams, ServerCapabilities, TextDocumentPositionParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};

use self::lookup::Found;
use self::position::{from_position, name_range, path_to_uri, uri_to_path, word_range};
use self::session::{ProjectState, Session};

pub fn run() -> Result<(), String> {
    let (connection, io_threads) = Connection::stdio();
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
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
    };
    server.serve()?;
    // The thread reading stdin only ends when the editor closes it, which
    // some editors do only after the server exits, so it is not waited for.
    drop(io_threads);
    Ok(())
}

fn workspace_root(params: &InitializeParams) -> Option<std::path::PathBuf> {
    let folder = params.workspace_folders.as_ref()?.first()?;
    uri_to_path(&folder.uri)
}

struct Server<'a> {
    connection: &'a Connection,
    session: Session,
}

impl Server<'_> {
    fn serve(&mut self) -> Result<(), String> {
        for message in &self.connection.receiver {
            match message {
                Message::Request(request) => {
                    let shutdown = self.connection.handle_shutdown(&request);
                    if shutdown.map_err(|error| error.to_string())? {
                        return Ok(());
                    }
                    self.request(request);
                }
                Message::Notification(notification) => self.notification(notification),
                Message::Response(_) => {}
            }
        }
        Ok(())
    }

    fn request(&mut self, request: Request) {
        let id = request.id.clone();
        let result = match request.method.as_str() {
            HoverRequest::METHOD => params::<HoverParams>(request)
                .map(|p| serde_json::to_value(self.hover(&p.text_document_position_params))),
            GotoDefinition::METHOD => params::<GotoDefinitionParams>(request)
                .map(|p| serde_json::to_value(self.definition(&p.text_document_position_params))),
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

    fn reply(&self, response: Response) {
        let _ = self.connection.sender.send(Message::Response(response));
    }

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
        if let Some(path) = changed {
            self.refresh(&path);
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
                    source: Some("b".to_string()),
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

    /// The project, the name under the cursor and the file's text.
    fn at<'s>(
        &'s self,
        at: &TextDocumentPositionParams,
    ) -> Option<(&'s ProjectState, Found<'s>, &'s str)> {
        let path = uri_to_path(&at.text_document.uri)?;
        let state = self.session.project_for(&path)?;
        let file = state.file_at(&path)?;
        let (line, column) = from_position(&file.text, at.position);
        let found = lookup::find_at(state, file.id, line, column)?;
        Some((state, found, &file.text))
    }

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

    fn definition(&self, at: &TextDocumentPositionParams) -> Option<GotoDefinitionResponse> {
        let (state, found, _) = self.at(at)?;
        let (span, length) = lookup::definition(state, &found)?;
        // The standard library has no file to open.
        let file = state.file(span.source)?;
        let uri: Uri = path_to_uri(&file.path)?;
        let range = name_range(&file.text, span, length);
        Some(GotoDefinitionResponse::Scalar(Location::new(uri, range)))
    }
}

fn params<P: serde::de::DeserializeOwned>(request: Request) -> Result<P, String> {
    let method = request.method.clone();
    let (_, params) = request
        .extract::<P>(&method)
        .map_err(|error: ExtractError<Request>| format!("{error:?}"))?;
    Ok(params)
}

fn notification_params<N: lsp_types::notification::Notification>(
    notification: Notification,
) -> Option<N::Params> {
    notification.extract(N::METHOD).ok()
}
