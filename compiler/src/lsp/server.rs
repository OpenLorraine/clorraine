//! The language server loop.
//!
//! Stdin is owned by a reader thread so a long analysis cannot stop the server
//! from reading a cancellation or a shutdown request. The reader recognizes
//! `$/cancelRequest` itself and records the id; the handler checks that set
//! before it starts work and between analysis files. Stdout is owned by the
//! handler, so responses and notifications never interleave mid-frame.

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::lsp::json::Json;
use crate::lsp::protocol::{FrameError, FrameReader, FrameWriter};
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
    let reader_cancelled = Arc::clone(&cancelled);
    thread::spawn(move || read_loop(sender, reader_cancelled));
    let mut server = Server {
        writer: FrameWriter::new(io::stdout()),
        cancelled,
        documents: DocumentStore::new(),
        index: ModuleIndex::default(),
        root: None,
        profile: Profile::parse("hosted-amiga").unwrap_or(Profile::Host),
        initialized: false,
        shutdown: false,
    };
    server.run(receiver)
}

/// Reads frames until end of stream or a fatal framing error.
fn read_loop(sender: mpsc::Sender<Incoming>, cancelled: Arc<Mutex<HashSet<String>>>) {
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
                            if let Ok(mut set) = cancelled.lock() {
                                set.insert(id);
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
    documents: DocumentStore,
    index: ModuleIndex,
    root: Option<PathBuf>,
    profile: Profile,
    initialized: bool,
    shutdown: bool,
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
            Some(id) => self.handle_request(&method, id, params),
            None => self.handle_notification(&method, params),
        }
    }

    /// Handles a request and writes its response.
    fn handle_request(&mut self, method: &str, id: Json, params: Json) -> Option<i32> {
        if let Some(key) = request_key(&id) {
            if self
                .cancelled
                .lock()
                .map(|set| set.contains(&key))
                .unwrap_or(false)
            {
                self.send_error(id, REQUEST_CANCELLED, "request cancelled");
                return None;
            }
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
            "textDocument/hover"
            | "textDocument/definition"
            | "textDocument/completion"
            | "textDocument/documentSymbol" => {
                let result = match method {
                    "textDocument/completion" | "textDocument/documentSymbol" => {
                        Json::Array(Vec::new())
                    }
                    _ => Json::Null,
                };
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
            "initialized" | "$/cancelRequest" | "textDocument/didSave" => None,
            "textDocument/didOpen" => {
                self.update_document(&params, DocumentChange::Open);
                None
            }
            "textDocument/didChange" => {
                self.update_document(&params, DocumentChange::Change);
                None
            }
            "textDocument/didClose" => {
                if let Some(uri) = document_uri(&params) {
                    self.documents.close(&uri);
                }
                None
            }
            _ => None,
        }
    }

    /// Applies an open or full-text change.
    fn update_document(&mut self, params: &Json, change: DocumentChange) {
        let Some(uri) = document_uri(params) else {
            return;
        };
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
        let Some(text) = text else {
            return;
        };
        match change {
            DocumentChange::Open => self.documents.open(&uri, text),
            DocumentChange::Change => self.documents.change(&uri, text),
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
        let _ = self.writer.write(&body.render());
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
        self.writer.write(&body.render()).is_ok()
    }
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
