//! The language server loop.
//!
//! Stdin is owned by a reader thread so a long analysis cannot stop the server
//! from reading a cancellation or a shutdown request. The reader recognizes
//! `$/cancelRequest` itself and records the id; the handler checks that set
//! before it starts work and between analysis files. Stdout is owned by the
//! handler, so responses and notifications never interleave mid-frame.

use std::collections::{BTreeSet, HashSet};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::analysis::{self, Mode};
use crate::diag::Severity;
use crate::lsp::features::{self, FeatureInput};
use crate::lsp::json::Json;
use crate::lsp::protocol::{FrameError, FrameReader, FrameWriter};
use crate::lsp::symbols::SymbolTable;
use crate::lsp::text::{LineIndex, Position};
use crate::lsp::uri::{self, DocumentUri};
use crate::lsp::workspace::{DocumentStore, ModuleIndex};
use crate::profile::Profile;

/// JSON-RPC parse error.
const PARSE_ERROR: i64 = -32700;
/// JSON-RPC invalid request.
const INVALID_REQUEST: i64 = -32600;
/// JSON-RPC method not found.
const METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC invalid params.
const INVALID_PARAMS: i64 = -32602;
/// LSP server not initialized.
const SERVER_NOT_INITIALIZED: i64 = -32002;
/// LSP request cancelled.
const REQUEST_CANCELLED: i64 = -32800;

/// One message from the reader thread.
enum Incoming {
    /// A parsed JSON value.
    Message(Json),
    /// A body that was not valid JSON.
    Malformed(String),
    /// A frame the reader could not recover from.
    BadFrame(String),
    /// Standard input closed.
    Eof,
}

/// Runs the server over the process standard streams.
pub fn run_stdio() -> i32 {
    let (sender, receiver) = mpsc::channel();
    let cancelled = Arc::new(Mutex::new(HashSet::new()));
    let completed = Arc::new(Mutex::new(HashSet::new()));
    let reader_cancelled = Arc::clone(&cancelled);
    let reader_completed = Arc::clone(&completed);
    thread::spawn(move || read_loop(sender, reader_cancelled, reader_completed));
    let mut server = Server {
        writer: FrameWriter::new(io::stdout()),
        cancelled,
        completed,
        documents: DocumentStore::new(),
        index: ModuleIndex::default(),
        root: None,
        profile: Profile::parse("hosted-amiga").unwrap_or(Profile::Host),
        initialized: false,
        shutdown: false,
        failed: false,
    };
    server.run(receiver)
}

/// Reads frames until end of stream or a fatal framing error.
fn read_loop(
    sender: mpsc::Sender<Incoming>,
    cancelled: Arc<Mutex<HashSet<String>>>,
    completed: Arc<Mutex<HashSet<String>>>,
) {
    let stdin = io::stdin();
    let mut reader = FrameReader::new(stdin.lock());
    loop {
        match reader.read() {
            Ok(Some(text)) => match Json::parse(&text) {
                Ok(value) => {
                    if value.get("method").and_then(Json::as_str) == Some("$/cancelRequest") {
                        if let Some(id) = value
                            .get("params")
                            .and_then(|params| params.get("id"))
                            .and_then(request_key)
                        {
                            let finished = completed
                                .lock()
                                .map(|set| set.contains(&id))
                                .unwrap_or(false);
                            if !finished {
                                if let Ok(mut set) = cancelled.lock() {
                                    if set.len() >= 1024 {
                                        set.clear();
                                    }
                                    set.insert(id);
                                }
                            }
                        }
                        continue;
                    }
                    if sender.send(Incoming::Message(value)).is_err() {
                        return;
                    }
                }
                Err(error) => {
                    if sender.send(Incoming::Malformed(error.message)).is_err() {
                        return;
                    }
                }
            },
            Ok(None) | Err(FrameError::Eof) => {
                let _ = sender.send(Incoming::Eof);
                return;
            }
            Err(error) => {
                let _ = sender.send(Incoming::BadFrame(format!("{error:?}")));
                return;
            }
        }
    }
}

/// The handler state.
struct Server<W: Write> {
    writer: FrameWriter<W>,
    cancelled: Arc<Mutex<HashSet<String>>>,
    completed: Arc<Mutex<HashSet<String>>>,
    documents: DocumentStore,
    index: ModuleIndex,
    root: Option<PathBuf>,
    profile: Profile,
    initialized: bool,
    shutdown: bool,
    failed: bool,
}

impl<W: Write> Server<W> {
    /// Runs the handler loop and returns the process exit code.
    fn run(&mut self, receiver: Receiver<Incoming>) -> i32 {
        while let Ok(incoming) = receiver.recv() {
            match incoming {
                Incoming::Message(value) => {
                    if let Some(code) = self.handle(value) {
                        return code;
                    }
                }
                Incoming::Malformed(message) => {
                    if !self.send_error(Json::Null, PARSE_ERROR, &message) {
                        return 1;
                    }
                }
                Incoming::BadFrame(message) => {
                    let _ = self.send_error(Json::Null, PARSE_ERROR, &message);
                    return 1;
                }
                Incoming::Eof => {
                    return if self.shutdown { 0 } else { 1 };
                }
            }
            if self.failed {
                eprintln!("clorraine-lsp: cannot write to stdout");
                return 1;
            }
        }
        0
    }

    /// Handles one message; returns an exit code when the server must stop.
    fn handle(&mut self, value: Json) -> Option<i32> {
        let method = value.get("method").and_then(Json::as_str).map(str::to_owned);
        let id = value.get("id").cloned();
        let params = value.get("params").cloned().unwrap_or(Json::Null);
        let Some(method) = method else {
            if let Some(id) = id {
                self.send_error(id, INVALID_REQUEST, "request has no method");
            }
            return None;
        };
        match id {
            Some(id) => {
                let key = request_key(&id);
                let code = self.handle_request(&method, id, params);
                if let Some(key) = key {
                    if let Ok(mut set) = self.completed.lock() {
                        if set.len() >= 4096 {
                            set.clear();
                        }
                        set.insert(key.clone());
                    }
                    if let Ok(mut set) = self.cancelled.lock() {
                        set.remove(&key);
                    }
                }
                code
            }
            None => self.handle_notification(&method, params),
        }
    }

    /// Handles a request and writes its response.
    fn handle_request(&mut self, method: &str, id: Json, params: Json) -> Option<i32> {
        if let Some(key) = request_key(&id) {
            let cancelled = self
                .cancelled
                .lock()
                .map(|mut set| set.remove(&key))
                .unwrap_or(false);
            if cancelled {
                self.send_error(id, REQUEST_CANCELLED, "request cancelled");
                return None;
            }
        }
        if self.shutdown {
            self.send_error(id, INVALID_REQUEST, "the server is shutting down");
            return None;
        }
        match method {
            "initialize" => {
                if self.initialized {
                    self.send_error(id, INVALID_REQUEST, "the server is already initialized");
                    return None;
                }
                self.initialize(id, params)
            }
            "shutdown" => {
                if !self.initialized {
                    self.send_error(id, SERVER_NOT_INITIALIZED, "send initialize first");
                    return None;
                }
                self.shutdown = true;
                self.send_result(id, Json::Null);
                None
            }
            _ if !self.initialized => {
                self.send_error(id, SERVER_NOT_INITIALIZED, "send initialize first");
                None
            }
            "textDocument/hover" => {
                let result = self
                    .request_position(&params)
                    .and_then(|(uri, position)| {
                        self.feature_input(&uri)
                            .and_then(|input| features::hover(&input, position))
                    })
                    .unwrap_or(Json::Null);
                self.send_result(id, result);
                None
            }
            "textDocument/definition" => {
                let result = self
                    .request_position(&params)
                    .and_then(|(uri, position)| {
                        self.feature_input(&uri)
                            .and_then(|input| features::definition(&input, position))
                    })
                    .unwrap_or(Json::Null);
                self.send_result(id, result);
                None
            }
            "textDocument/completion" => {
                let modules: BTreeSet<String> =
                    self.index.module_names().cloned().collect();
                let result = self
                    .request_uri(&params)
                    .and_then(|uri| self.feature_input(&uri))
                    .map(|input| features::completion(&input, &modules))
                    .unwrap_or_else(|| Json::Array(Vec::new()));
                self.send_result(id, result);
                None
            }
            "textDocument/documentSymbol" => {
                let result = self
                    .request_uri(&params)
                    .and_then(|uri| self.feature_input(&uri))
                    .map(|input| features::document_symbols(&input))
                    .unwrap_or(Json::Null);
                self.send_result(id, result);
                None
            }
            _ => {
                self.send_error(id, METHOD_NOT_FOUND, &format!("unknown method '{method}'"));
                None
            }
        }
    }

    /// Handles one notification; only `exit` stops the server.
    fn handle_notification(&mut self, method: &str, params: Json) -> Option<i32> {
        match method {
            "exit" => Some(if self.shutdown { 0 } else { 1 }),
            "initialized" | "$/cancelRequest" => None,
            _ if !self.initialized || self.shutdown => None,
            "textDocument/didOpen" => {
                if let Some(uri) = self.update_document(&params, DocumentChange::Open) {
                    self.publish_diagnostics(&uri);
                }
                None
            }
            "textDocument/didChange" => {
                if let Some(uri) = self.update_document(&params, DocumentChange::Change) {
                    self.publish_diagnostics(&uri);
                }
                None
            }
            "textDocument/didSave" => {
                if let Some(uri) = document_uri(&params) {
                    if let Some(text) = params.get("text").and_then(Json::as_str) {
                        self.documents.change(&uri, text.to_owned());
                    }
                    self.publish_diagnostics(&uri);
                }
                None
            }
            "textDocument/didClose" => {
                if let Some(uri) = document_uri(&params) {
                    self.documents.close(&uri);
                    self.publish_empty(&uri);
                }
                None
            }
            _ => None,
        }
    }

    /// Applies an open or full-text change.
    fn update_document(
        &mut self,
        params: &Json,
        change: DocumentChange,
    ) -> Option<DocumentUri> {
        let uri = document_uri(params)?;
        let text = match change {
            DocumentChange::Open => params
                .get("textDocument")
                .and_then(|document| document.get("text"))
                .and_then(Json::as_str)
                .map(str::to_owned),
            DocumentChange::Change => params
                .get("contentChanges")
                .and_then(Json::as_array)
                .and_then(|changes| changes.last())
                .and_then(|change| change.get("text"))
                .and_then(Json::as_str)
                .map(str::to_owned),
        };
        let text = text?;
        match change {
            DocumentChange::Open => self.documents.open(&uri, text),
            DocumentChange::Change => self.documents.change(&uri, text),
        }
        Some(uri)
    }

    /// Parses the analysis set for one requested document.
    fn feature_input(&self, uri: &DocumentUri) -> Option<FeatureInput> {
        let set = self.index.analysis_set(uri, &self.documents);
        let parsed = analysis::parse_partial(set);
        let file = *parsed.files.get(&uri.key())?;
        let table = SymbolTable::build(&parsed.sources, &parsed.programs);
        Some(FeatureInput {
            sources: parsed.sources,
            file,
            table,
        })
    }

    /// Extracts the document from a feature request.
    fn request_uri(&self, params: &Json) -> Option<DocumentUri> {
        document_uri(params)
    }

    /// Extracts the document and position from a feature request.
    fn request_position(&self, params: &Json) -> Option<(DocumentUri, Position)> {
        let uri = document_uri(params)?;
        let position = params.get("position")?;
        let line = position.get("line")?.as_i64()?;
        let character = position.get("character")?.as_i64()?;
        if line < 0 || character < 0 {
            return None;
        }
        Some((
            uri,
            Position {
                line: line as u32,
                character: character as u32,
            },
        ))
    }

    /// Analyzes one document and publishes its diagnostics.
    fn publish_diagnostics(&mut self, uri: &DocumentUri) {
        let key = uri.key();
        let set = self.index.analysis_set(uri, &self.documents);
        let analysis = analysis::analyze(self.profile, set, Mode::Full, false);
        let mut diagnostics = Vec::new();
        if let Some(file) = analysis.files.get(&key).copied() {
            let text = analysis.sources.text(file);
            let index = LineIndex::new(text);
            for diagnostic in analysis.diagnostics.items() {
                let Some(span) = diagnostic.span else {
                    continue;
                };
                if span.file != file {
                    continue;
                }
                let mut message = diagnostic.message.clone();
                if let Some(help) = &diagnostic.help {
                    message.push_str("\n\n");
                    message.push_str(help);
                }
                let severity = match diagnostic.severity {
                    Severity::Error => 1,
                    Severity::Warning => 2,
                };
                diagnostics.push(Json::object([
                    ("range", range_json(index.range(text, span))),
                    ("severity", Json::Number(severity)),
                    ("code", Json::String(diagnostic.code.to_owned())),
                    ("source", Json::String("clorraine".to_owned())),
                    ("message", Json::String(message)),
                ]));
            }
        }
        self.send_notification(
            "textDocument/publishDiagnostics",
            Json::object([
                ("uri", Json::String(uri.to_uri())),
                ("diagnostics", Json::Array(diagnostics)),
            ]),
        );
    }

    /// Publishes an empty diagnostic list for one document.
    fn publish_empty(&mut self, uri: &DocumentUri) {
        self.send_notification(
            "textDocument/publishDiagnostics",
            Json::object([
                ("uri", Json::String(uri.to_uri())),
                ("diagnostics", Json::Array(Vec::new())),
            ]),
        );
    }

    /// Writes one server notification.
    fn send_notification(&mut self, method: &str, params: Json) {
        let body = Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("method", Json::String(method.to_owned())),
            ("params", params),
        ]);
        if self.writer.write(&body.render()).is_err() {
            self.failed = true;
        }
    }

    /// Handles `initialize`.
    fn initialize(&mut self, id: Json, params: Json) -> Option<i32> {
        let profile_name = params
            .get("initializationOptions")
            .and_then(|options| options.get("profile"))
            .and_then(Json::as_str)
            .unwrap_or("hosted-amiga");
        let Some(profile) = Profile::parse(profile_name) else {
            self.send_error(
                id,
                INVALID_PARAMS,
                &format!(
                    "unknown profile '{profile_name}'; use hosted-amiga, hosted-amiga-13, host, or freestanding"
                ),
            );
            return None;
        };
        if let Some(root) = root_from_params(&params) {
            match root {
                Ok(path) => {
                    self.root = Some(path.clone());
                    self.index = ModuleIndex::index_root(&path);
                }
                Err(message) => {
                    self.send_error(id, INVALID_PARAMS, &message);
                    return None;
                }
            }
        }
        self.profile = profile;
        self.initialized = true;
        let capabilities = Json::object([
            ("positionEncoding", Json::String("utf-16".to_owned())),
            ("textDocumentSync", Json::Number(1)),
            ("hoverProvider", Json::Bool(true)),
            ("definitionProvider", Json::Bool(true)),
            (
                "completionProvider",
                Json::object([(
                    "triggerCharacters",
                    Json::Array(vec![
                        Json::String(".".to_owned()),
                        Json::String(":".to_owned()),
                    ]),
                )]),
            ),
            ("documentSymbolProvider", Json::Bool(true)),
        ]);
        let result = Json::object([
            ("capabilities", capabilities),
            (
                "serverInfo",
                Json::object([
                    ("name", Json::String("clorraine-lsp".to_owned())),
                    (
                        "version",
                        Json::String(env!("CARGO_PKG_VERSION").to_owned()),
                    ),
                ]),
            ),
        ]);
        self.send_result(id, result);
        None
    }

    /// Writes a successful response.
    fn send_result(&mut self, id: Json, result: Json) {
        let body = Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("id", id),
            ("result", result),
        ]);
        if self.writer.write(&body.render()).is_err() {
            self.failed = true;
        }
    }

    /// Writes an error response; returns false when the stream failed.
    fn send_error(&mut self, id: Json, code: i64, message: &str) -> bool {
        let body = Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("id", id),
            (
                "error",
                Json::object([
                    ("code", Json::Number(code)),
                    ("message", Json::String(message.to_owned())),
                ]),
            ),
        ]);
        let written = self.writer.write(&body.render()).is_ok();
        if !written {
            self.failed = true;
        }
        written
    }
}

/// Renders a protocol range.
fn range_json(range: crate::lsp::text::Range) -> Json {
    Json::object([
        ("start", position_json(range.start)),
        ("end", position_json(range.end)),
    ])
}

/// Renders a protocol position.
fn position_json(position: Position) -> Json {
    Json::object([
        ("line", Json::Number(position.line as i64)),
        ("character", Json::Number(position.character as i64)),
    ])
}

/// Returns the cancellation key for a request id.
///
/// LSP allows a number or a string; both are kept distinct by the prefix.
fn request_key(id: &Json) -> Option<String> {
    match id {
        Json::Number(value) => Some(format!("n:{value}")),
        Json::String(value) => Some(format!("s:{value}")),
        _ => None,
    }
}

/// The kind of document update.
#[derive(Clone, Copy)]
enum DocumentChange {
    Open,
    Change,
}

/// Extracts and parses the document URI from `textDocument.uri`.
fn document_uri(params: &Json) -> Option<DocumentUri> {
    let text = params
        .get("textDocument")
        .and_then(|document| document.get("uri"))
        .and_then(Json::as_str)?;
    uri::parse(text).ok()
}

/// Extracts the workspace root from `rootUri` or `rootPath`.
fn root_from_params(params: &Json) -> Option<Result<PathBuf, String>> {
    if let Some(root) = params.get("rootUri").and_then(Json::as_str) {
        return Some(match uri::parse(root) {
            Ok(DocumentUri::File(path)) => Ok(path),
            Ok(DocumentUri::Untitled(_)) => Err("rootUri must be a file URI".to_owned()),
            Err(message) => Err(message),
        });
    }
    params
        .get("rootPath")
        .and_then(Json::as_str)
        .map(|path| Ok(PathBuf::from(path)))
}
