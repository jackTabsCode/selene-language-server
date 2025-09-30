use tower_lsp::jsonrpc::Result;
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
            .log_message(MessageType::INFO, "server initialized!")
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
                let title = format!("Allow rule {code} for this line");

                let pos = Position::new(diagnostic.range.start.line, 0);

                let edit = TextEdit {
                    range: Range::new(pos, pos),
                    new_text: format!("-- selene: allow({code})\n"),
                };

                let edit_workspace = WorkspaceEdit {
                    changes: Some(
                        [(params.text_document.uri.clone(), vec![edit])]
                            .into_iter()
                            .collect(),
                    ),
                    ..Default::default()
                };

                let action = CodeAction {
                    title,
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: Some(edit_workspace),
                    command: None,
                    is_preferred: Some(true),
                    disabled: None,
                    data: None,
                };

                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        Ok(Some(actions))
    }

    async fn shutdown(&self) -> Result<()> {
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
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
