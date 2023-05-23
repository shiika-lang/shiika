use crate::server::{MsgFromServer, MsgToServer, Server};
use async_channel::{unbounded, Receiver, Sender};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use std::io::Write;
use std::fs::OpenOptions;

const COMPLETION_TRIGGER: &[&str] = &["<", ">", "=", "!"];

#[derive(Debug)]
pub struct Backend {
    client: Client,
    rcv: Receiver<MsgFromServer>,
    snd: Sender<MsgToServer>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let (tx_from, rx_from) = unbounded();
        let (tx_to, rx_to) = unbounded();
        let mut server = Server::new(client.clone(), rx_to, tx_from);
        std::thread::spawn(move || server.serve());

        Self {
            client,
            rcv: rx_from,
            snd: tx_to,
        }
    }

    async fn send(&self, msg: MsgToServer) {
        if let Err(x) = self.snd.send(msg).await {
            self.client.log_message(MessageType::ERROR, x).await;
        }
    }

    async fn recv(&self) -> Option<MsgFromServer> {
        match self.rcv.recv().await {
            Ok(x) => Some(x),
            Err(x) => {
                self.client.log_message(MessageType::ERROR, x).await;
                None
            }
        }
    }

    /// Write debug log
    fn log(s: &str) {
//        let mut file = OpenOptions::new().append(true).open("/Users/yhara/tmp/skc_language_server.log").unwrap();
//        file.write_all(s.as_bytes()).unwrap();
//        file.write_all(b"\n").unwrap();
//        file.flush().unwrap();
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    /// Returns capabilities of this language server.
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Backend::log("- Backend::initialize");
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
//                text_document_sync: Some(TextDocumentSyncCapability::Kind(
//                    TextDocumentSyncKind::FULL,
//                )),
//                workspace: Some(WorkspaceServerCapabilities {
//                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
//                        supported: Some(true),
//                        change_notifications: Some(OneOf::Left(true)),
//                    }),
//                    file_operations: None,
//                }),
                definition_provider: Some(OneOf::Left(true)),
//                document_formatting_provider: Some(OneOf::Left(true)),
//                workspace_symbol_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
//                semantic_tokens_provider: Some(
//                    SemanticTokensServerCapabilities::SemanticTokensOptions(
//                        SemanticTokensOptions {
//                            work_done_progress_options: WorkDoneProgressOptions {
//                                work_done_progress: Some(false),
//                            },
//                            legend: SemanticTokensLegend {
//                                token_types: semantic_legend::get_token_types(),
//                                token_modifiers: semantic_legend::get_token_modifiers(),
//                            },
//                            range: Some(false),
//                            full: Some(SemanticTokensFullOptions::Delta { delta: Some(false) }),
//                        },
//                    ),
//                ),
//                completion_provider: Some(CompletionOptions {
//                    resolve_provider: Some(false),
//                    trigger_characters: Some(
//                        COMPLETION_TRIGGER.iter().map(|x| x.to_string()).collect(),
//                    ),
//                    all_commit_characters: None,
//                    work_done_progress_options: WorkDoneProgressOptions::default(),
//                    completion_item: None,
//                }),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "skc_language_server".to_string()),
                version: Some(String::from(env!("CARGO_PKG_VERSION"))),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        Backend::log("- Backend::initialized");
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "did_open").await;

        let url = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;
        Backend::log(&format!("- Backend::did_open(url: {}, text: {}, version: {})",
                             url, text, version));

        self.send(MsgToServer::DidOpen { url, text, version }).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "did_change")
            .await;

        let url = params.text_document.uri;
        let text = std::mem::take(&mut params.content_changes[0].text);
        let version = params.text_document.version;
        Backend::log(&format!("- Backend::did_change(url: {}, text: {}, version: {}",
                             url, text, version));

        self.send(MsgToServer::DidChange { url, text, version })
            .await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for change in params.changes {
            self.client
                .log_message(
                    MessageType::INFO,
                    format!("did_change_watched_files: {change:?}"),
                )
                .await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let url = params.text_document_position.text_document.uri;
        let line = params.text_document_position.position.line as usize + 1;
        let column = params.text_document_position.position.character as usize + 1;
        let context = params.context;

        self.send(MsgToServer::Completion {
            url,
            line,
            column,
            context,
        })
        .await;

        if let Some(MsgFromServer::Completion(x)) = self.recv().await {
            Ok(x)
        } else {
            Ok(None)
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let url = params.text_document_position_params.text_document.uri;
        let line = params.text_document_position_params.position.line as usize + 1;
        let column = params.text_document_position_params.position.character as usize + 1;

        self.send(MsgToServer::GotoDefinition { url, line, column })
            .await;

        if let Some(MsgFromServer::GotoDefinition(Some(x))) = self.recv().await {
            Ok(Some(GotoDefinitionResponse::Scalar(x)))
        } else {
            Ok(None)
        }
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let query = params.query;

        self.send(MsgToServer::Symbol { query }).await;

        if let Some(MsgFromServer::Symbol(x)) = self.recv().await {
            Ok(Some(x))
        } else {
            Ok(None)
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let url = params.text_document_position_params.text_document.uri;
        let line = params.text_document_position_params.position.line as usize + 1;
        let column = params.text_document_position_params.position.character as usize + 1;
        Backend::log(&format!("- Backend::hover(url: {}, line: {}, column: {})",
                             url, line, column));

        self.send(MsgToServer::Hover { url, line, column }).await;

        if let Some(MsgFromServer::Hover(Some(x))) = self.recv().await {
            Ok(Some(x))
        } else {
            Ok(None)
        }
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let url = params.text_document_position.text_document.uri;
        let line = params.text_document_position.position.line as usize + 1;
        let column = params.text_document_position.position.character as usize + 1;
        Backend::log(&format!("- Backend::references(url: {}, line: {}, column: {}",
                             url, line, column));

        self.send(MsgToServer::References { url, line, column })
            .await;

        if let Some(MsgFromServer::References(x)) = self.recv().await {
            Ok(Some(x))
        } else {
            Ok(None)
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let url = params.text_document.uri;

        self.send(MsgToServer::SemanticTokens { url }).await;

        if let Some(MsgFromServer::SemanticTokens(x)) = self.recv().await {
            Ok(x)
        } else {
            Ok(None)
        }
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let url = params.text_document.uri;

        self.send(MsgToServer::Formatting { url }).await;

        if let Some(MsgFromServer::Formatting(x)) = self.recv().await {
            Ok(x)
        } else {
            Ok(None)
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Backend::log(&format!("- Backend::shutdown"));
        Ok(())
    }
}
