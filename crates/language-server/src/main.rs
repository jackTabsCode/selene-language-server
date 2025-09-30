use tokio::process;
use tower_lsp::jsonrpc::{self, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::selene::run_selene;

mod selene;

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        match process::Command::new("selene")
            .arg("--version")
            .output()
            .await
        {
            Ok(output) => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                self.client
                    .log_message(MessageType::INFO, format!("Found {version}"))
                    .await;
            }
            Err(err) => {
                return Err(jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: format!("Failed to run selene: {err}").into(),
                    data: None,
                });
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // Selene can only accept full files
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Server initialized!")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.lint(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // We just grab the first change, since there should only be one
        if let Some(change) = params.content_changes.into_iter().next() {
            self.lint(params.text_document.uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let mut actions = Vec::new();

        for diagnostic in &params.context.diagnostics {
            if diagnostic.source.as_deref() == Some("selene")
                && let Some(NumberOrString::String(code)) = &diagnostic.code
            {
                for entire_file in [true, false] {
                    actions.push(self.make_allow_action(
                        diagnostic,
                        &params.text_document.uri,
                        code,
                        entire_file,
                    ));
                }
            }
        }

        Ok(Some(actions))
    }

    async fn shutdown(&self) -> Result<()> {
        self.client
            .log_message(MessageType::INFO, "Server shutting down!")
            .await;

        Ok(())
    }
}

impl Backend {
    async fn lint(&self, uri: Url, text: String) {
        let diagnostics = run_selene(&text).await;

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    fn make_allow_action(
        &self,
        diagnostic: &Diagnostic,
        uri: &Url,
        code: &str,
        entire_file: bool,
    ) -> CodeActionOrCommand {
        let (pos, prefix, title) = if entire_file {
            (
                Position::new(0, 0),
                "--#",
                format!("Allow rule {code} for the entire file"),
            )
        } else {
            (
                Position::new(diagnostic.range.start.line, 0),
                "--",
                format!("Allow rule {code} for this line"),
            )
        };

        let edit = TextEdit {
            range: Range::new(pos, pos),
            new_text: format!("{prefix} selene: allow({code})\n"),
        };

        let edit_workspace = WorkspaceEdit {
            changes: Some([(uri.clone(), vec![edit])].into_iter().collect()),
            ..Default::default()
        };

        CodeActionOrCommand::CodeAction(CodeAction {
            title,
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(edit_workspace),
            command: None,
            is_preferred: Some(!entire_file),
            disabled: None,
            data: None,
        })
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
