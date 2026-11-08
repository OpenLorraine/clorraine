//! A small blocking LSP client for the integration tests.
//!
//! The client spawns the real server binary, frames every message, and reads
//! responses on a helper thread. Every wait has a deadline, so a server bug
//! fails the test instead of hanging the suite. The child is killed when the
//! client is dropped.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use clc::lsp::json::Json;

/// Longest wait for one response.
const TIMEOUT: Duration = Duration::from_secs(10);

/// A client connected to one server process.
pub struct LspClient {
    child: Child,
    stdin: ChildStdin,
    receiver: Receiver<Result<String, String>>,
    pending: Vec<Json>,
    next_id: i64,
}

impl LspClient {
    /// Starts the server with no workspace root.
    pub fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_clorraine-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn clorraine-lsp");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_frame(&mut reader) {
                    Ok(Some(text)) => {
                        if sender.send(Ok(text)).is_err() {
                            return;
                        }
                    }
                    Ok(None) => {
                        let _ = sender.send(Err("server closed stdout".to_owned()));
                        return;
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        return;
                    }
                }
            }
        });
        Self {
            child,
            stdin,
            receiver,
            pending: Vec::new(),
            next_id: 1,
        }
    }

    /// Sends one already-rendered JSON value.
    pub fn send_text(&mut self, text: &str) {
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", text.len(), text)
            .expect("write frame");
        self.stdin.flush().expect("flush frame");
    }

    /// Sends one value.
    pub fn send(&mut self, value: &Json) {
        self.send_text(&value.render());
    }

    /// Sends a request with a fresh id and returns the response value.
    pub fn request(&mut self, method: &str, params: Json) -> Json {
        let id = self.next_id;
        self.next_id += 1;
        let request = Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("id", Json::Number(id)),
            ("method", Json::String(method.to_owned())),
            ("params", params),
        ]);
        self.send(&request);
        self.await_response(id)
    }

    /// Sends a request with an explicit id.
    pub fn request_with_id(&mut self, id: i64, method: &str, params: Json) {
        let request = Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("id", Json::Number(id)),
            ("method", Json::String(method.to_owned())),
            ("params", params),
        ]);
        self.send(&request);
    }

    /// Reads the next message, including notifications.
    pub fn message(&mut self) -> Json {
        if !self.pending.is_empty() {
            return self.pending.remove(0);
        }
        let text = self.next_frame();
        Json::parse(&text).expect("server sent valid JSON")
    }

    /// Waits for the response with `id`, buffering notifications.
    pub fn await_response(&mut self, id: i64) -> Json {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(index) = self
                .pending
                .iter()
                .position(|value| value.get("id").and_then(Json::as_i64) == Some(id))
            {
                return self.pending.remove(index);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                let _ = self.child.kill();
                panic!("no response to request {id} within {TIMEOUT:?}");
            }
            match self.receiver.recv_timeout(remaining) {
                Ok(Ok(text)) => {
                    let value = Json::parse(&text).expect("server sent valid JSON");
                    if value.get("id").and_then(Json::as_i64) == Some(id) {
                        return value;
                    }
                    self.pending.push(value);
                }
                Ok(Err(error)) => {
                    let _ = self.child.kill();
                    panic!("server output ended: {error}");
                }
                Err(RecvTimeoutError::Timeout) => {
                    let _ = self.child.kill();
                    panic!("no response to request {id} within {TIMEOUT:?}");
                }
                Err(RecvTimeoutError::Disconnected) => {
                    let _ = self.child.kill();
                    panic!("server output thread stopped");
                }
            }
        }
    }

    /// Initializes with a profile and no workspace root.
    pub fn initialize(&mut self, profile: &str) -> Json {
        self.request(
            "initialize",
            Json::object([(
                "initializationOptions",
                Json::object([("profile", Json::String(profile.to_owned()))]),
            )]),
        )
    }

    /// Shuts down and sends `exit`; returns the process exit code.
    pub fn shutdown_and_exit(&mut self) -> i32 {
        let response = self.request("shutdown", Json::Null);
        assert!(response.get("error").is_none(), "shutdown failed: {response:?}");
        self.send_text(&Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("method", Json::String("exit".to_owned())),
        ])
        .render());
        self.wait_exit()
    }

    /// Waits for the process to exit, killing it after the deadline.
    pub fn wait_exit(&mut self) -> i32 {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return status.code().unwrap_or(-1),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => {
                    let _ = self.child.kill();
                    panic!("server did not exit within {TIMEOUT:?}");
                }
            }
        }
    }

    /// Reads the next framed body.
    fn next_frame(&mut self) -> String {
        let deadline = Instant::now() + TIMEOUT;
        let remaining = deadline.saturating_duration_since(Instant::now());
        match self.receiver.recv_timeout(remaining) {
            Ok(Ok(text)) => text,
            Ok(Err(error)) => panic!("server output ended: {error}"),
            Err(error) => panic!("no server message: {error}"),
        }
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Reads one frame; `Ok(None)` means a clean end of stream.
fn read_frame(reader: &mut BufReader<std::process::ChildStdout>) -> Result<Option<String>, String> {
    let mut length: Option<usize> = None;
    let mut saw_header = false;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).map_err(|error| error.to_string())?;
        if read == 0 {
            return if saw_header {
                Err("truncated header".to_owned())
            } else {
                Ok(None)
            };
        }
        saw_header = true;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| "bad content length".to_owned())?,
                );
            }
        }
    }
    let length = length.ok_or_else(|| "missing content length".to_owned())?;
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|error| error.to_string())?;
    String::from_utf8(body)
        .map(Some)
        .map_err(|_| "body is not UTF-8".to_owned())
}

/// Extracts the numeric error code from a response.
pub fn error_code(value: &Json) -> Option<i64> {
    value.get("error")?.get("code")?.as_i64()
}
