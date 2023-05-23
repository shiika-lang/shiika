use async_channel::{Receiver, Sender};
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

pub enum MsgToServer {
    DidOpen {
        url: Url,
        text: String,
        version: i32,
    },
    DidChange {
        url: Url,
        text: String,
        version: i32,
    },
    Completion {
        url: Url,
        line: usize,
        column: usize,
        context: Option<CompletionContext>,
    },
    GotoDefinition {
        url: Url,
        line: usize,
        column: usize,
    },
    Symbol {
        query: String,
    },
    Hover {
        url: Url,
        line: usize,
        column: usize,
    },
    References {
        url: Url,
        line: usize,
        column: usize,
    },
    SemanticTokens {
        url: Url,
    },
    Formatting {
        url: Url,
    },
}

pub enum MsgFromServer {
    Completion(Option<CompletionResponse>),
    GotoDefinition(Option<Location>),
    Symbol(Vec<SymbolInformation>),
    Hover(Option<Hover>),
    References(Vec<Location>),
    SemanticTokens(Option<SemanticTokensResult>),
    Formatting(Option<Vec<TextEdit>>),
}

pub struct Server {
    client: Client,
    rcv: Receiver<MsgToServer>,
    snd: Sender<MsgFromServer>,
//    document_map: DashMap<String, Rope>,
//    parser_map: DashMap<String, Parser>,
//    metadata_map: DashMap<String, Metadata>,
//    cache_dir: String,
//    lsp_token: i32,
//    background_tasks: VecDeque<BackgroundTask>,
}

impl Server {
    pub fn new(client: Client, rcv: Receiver<MsgToServer>, snd: Sender<MsgFromServer>) -> Self {
        Server {
            client,
            rcv,
            snd,
//            document_map: DashMap::new(),
//            parser_map: DashMap::new(),
//            metadata_map: DashMap::new(),
//            cache_dir: Metadata::cache_dir().to_string_lossy().to_string(),
//            lsp_token: 0,
//            background_tasks: VecDeque::new(),
        }
    }

    pub fn serve(&mut self) {
        loop {
            if let Ok(msg) = self.rcv.recv_blocking() {
                match msg {
                    MsgToServer::DidOpen { url, text, version } => {
                        self.did_open(&url, &text, version)
                    }
                    MsgToServer::DidChange { url, text, version } => {
                        self.did_change(&url, &text, version)
                    }
                    MsgToServer::Completion {
                        url,
                        line,
                        column,
                        context,
                    } => self.completion(&url, line, column, &context),
                    MsgToServer::GotoDefinition { url, line, column } => {
                        self.goto_definition(&url, line, column)
                    }
                    MsgToServer::Symbol { query } => self.symbol(&query),
                    MsgToServer::Hover { url, line, column } => self.hover(&url, line, column),
                    MsgToServer::References { url, line, column } => {
                        self.references(&url, line, column)
                    }
                    MsgToServer::SemanticTokens { url } => self.semantic_tokens(&url),
                    MsgToServer::Formatting { url } => self.formatting(&url),
                }
            }

//            while self.rcv.is_empty() && !self.background_tasks.is_empty() {
//                if let Some(mut task) = self.background_tasks.pop_front() {
//                    if !task.progress {
//                        self.progress_start("background analyze");
//                        task.progress = true;
//                    }
//                    if let Some(path) = task.paths.pop() {
//                        self.background_analyze(&path, &task.metadata);
//                        let pcnt = (task.total - task.paths.len()) * 100 / task.total;
//                        self.progress_report(
//                            &format!("{}", path.src.file_name().unwrap().to_string_lossy()),
//                            pcnt as u32,
//                        );
//                    }
//                    if task.paths.is_empty() {
//                        self.progress_done("background analyze done");
//                    } else {
//                        self.background_tasks.push_front(task);
//                    }
//                }
//            }
        }
    }

    fn did_open(&mut self, url: &Url, text: &str, version: i32) {
    }

    fn did_change(&mut self, url: &Url, text: &str, version: i32) {
    }

    fn completion(
        &mut self,
        url: &Url,
        line: usize,
        column: usize,
        context: &Option<CompletionContext>,
    ) {
    }

    fn goto_definition(&mut self, url: &Url, line: usize, column: usize) {
    }

    fn symbol(&mut self, query: &str) {
    }

    fn hover(&mut self, url: &Url, line: usize, column: usize) {
        let msg = format!("{}:{}", &line, &column);
        let hover = Hover {
            contents: HoverContents::Scalar(MarkedString::String(msg)),
            range: None,
        };
        self.snd
            .send_blocking(MsgFromServer::Hover(Some(hover)))
            .unwrap();
    }

    fn references(&mut self, url: &Url, line: usize, column: usize) {
    }

    fn semantic_tokens(&mut self, url: &Url) {
    }

    fn formatting(&mut self, url: &Url) {
    }
}

