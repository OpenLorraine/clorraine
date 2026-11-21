//! Process contracts for the exact version query and unchanged stdio startup.

use clc::lsp::json::Json;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Allow process startup on a busy test host without waiting indefinitely.
const DEADLINE: Duration = Duration::from_secs(10);
/// These tiny lifecycle exchanges must not allocate unbounded captured output.
const OUTPUT_LIMIT: u64 = 64 * 1024;

/// Keep stdin untouched while waiting; kill and reap before reporting a timeout.
fn wait(child: &mut Child) -> (bool, ExitStatus) {
    let deadline = Instant::now() + DEADLINE;
    loop {
        if let Some(status) = child.try_wait().expect("inspect server status") {
            return (false, status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return (true, child.wait().expect("join timed-out server"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Own one server and file-backed streams, including cleanup on assertion failure.
///
/// File-backed output avoids blocked pipe readers during the open-stdin test.
/// This small argument-aware probe does not replace the full protocol oracle.
struct Session {
    root: PathBuf,
    child: Option<Child>,
    offset: usize,
}

impl Session {
    /// Start the real entry with literal arguments and optionally retain stdin.
    fn start(arguments: &[&str], keep_stdin_open: bool) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::var_os("CODEOPS_TMPDIR").map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build/tmp"));
        std::fs::create_dir_all(&base).expect("create scratch parent");
        let root = loop {
            let path = base.join(format!("native-lsp-version-{}-{}",
                std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            match std::fs::create_dir(&path) {
                Ok(()) => break path,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create owned scratch: {error}"),
            }
        };
        let mut session = Self { root, child: None, offset: 0 };
        session.child = Some(Command::new(env!("CARGO_BIN_EXE_clorraine-lsp"))
            .args(arguments).stdin(Stdio::piped())
            .stdout(File::create(session.root.join("stdout")).unwrap())
            .stderr(File::create(session.root.join("stderr")).unwrap())
            .spawn().expect("start language server"));
        if !keep_stdin_open {
            drop(session.child.as_mut().unwrap().stdin.take());
        }
        session
    }

    /// Read a bounded stream without depending on EOF from a live server.
    fn captured(&self, stream: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        File::open(self.root.join(stream)).unwrap().take(OUTPUT_LIMIT + 1)
            .read_to_end(&mut bytes).unwrap();
        assert!(bytes.len() as u64 <= OUTPUT_LIMIT, "excessive {stream}");
        bytes
    }

    /// Write only standard Content-Length framing and static lifecycle messages.
    fn send(&mut self, text: &str) {
        let stdin = self.child.as_mut().unwrap().stdin.as_mut().unwrap();
        write!(stdin, "Content-Length: {}\r\n\r\n{}", text.len(), text).unwrap();
        stdin.flush().unwrap();
    }

    /// Await one response, allowing notifications as the existing client does.
    fn response(&mut self, id: i64) -> Json {
        let deadline = Instant::now() + DEADLINE;
        loop {
            let bytes = self.captured("stdout");
            let unread = &bytes[self.offset..];
            if let Some(end) = unread.windows(4).position(|part| part == b"\r\n\r\n") {
                let header = std::str::from_utf8(&unread[..end]).unwrap();
                let length: usize = header.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.trim().eq_ignore_ascii_case("content-length").then_some(value.trim())
                }).expect("protocol response header").parse().unwrap();
                assert!(length as u64 <= OUTPUT_LIMIT);
                let body = end + 4;
                if unread.len() - body >= length {
                    let response = Json::parse(std::str::from_utf8(&unread[body..body + length])
                        .unwrap()).expect("valid protocol response");
                    self.offset += body + length;
                    if response.get("id").and_then(Json::as_i64) == Some(id) {
                        return response;
                    }
                    assert!(Instant::now() < deadline, "protocol response deadline");
                    continue;
                }
            }
            assert!(self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "server exited before a protocol response: {bytes:?}");
            assert!(Instant::now() < deadline, "protocol response deadline");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Join before returning any result that a test will assert, even on timeout.
    fn finish(&mut self) -> (bool, Output) {
        let (timed_out, status) = wait(self.child.as_mut().unwrap());
        (timed_out, Output { status, stdout: self.captured("stdout"),
            stderr: self.captured("stderr") })
    }
}

impl Drop for Session {
    /// Never leave a server alive or delete its streams before it has stopped.
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Require a plain exact banner, without protocol frames or error output.
fn version_query(keep_stdin_open: bool) {
    let mut session = Session::start(&["--version"], keep_stdin_open);
    let (timed_out, output) = session.finish();
    assert!(!timed_out, "version query waited for input or a protocol handshake");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, format!("clorraine-lsp {}\n", env!("CARGO_PKG_VERSION")).as_bytes());
    assert!(output.stderr.is_empty(), "{output:?}");
}

/// EOF must not be required to obtain the ordinary version-query result.
#[test]
fn exact_version_with_closed_stdin_prints_current_banner() {
    version_query(false);
}

/// An idle open pipe must not turn an informational command into an LSP session.
#[test]
fn exact_version_with_open_stdin_exits_without_protocol_handshake() {
    version_query(true);
}

/// Only the exact one-argument query bypasses initialize, shutdown and exit.
#[test]
fn other_argument_vectors_preserve_stdio_lifecycle() {
    let cases: &[&[&str]] = &[
        &[], &["--unknown-lsp-option"], &["--help"], &["--version", "extra"],
        &["extra", "--version"], &["--version", "--version"], &["--version=1"],
    ];
    for arguments in cases {
        let mut session = Session::start(arguments, true);
        session.send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"initializationOptions":{"profile":"host"}}}"#);
        let initialized = session.response(1);
        assert_eq!(initialized.get("id"), Some(&Json::Number(1)), "{arguments:?}");
        let result = initialized.get("result").expect("initialize result");
        assert_eq!(result.get("serverInfo").and_then(|info| info.get("name")).and_then(Json::as_str),
            Some("clorraine-lsp"), "{arguments:?}");
        session.send(r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}"#);
        let shutdown = session.response(2);
        assert_eq!(shutdown.get("id"), Some(&Json::Number(2)), "{arguments:?}");
        assert_eq!(shutdown.get("result"), Some(&Json::Null), "{arguments:?}");
        assert!(shutdown.get("error").is_none(), "{arguments:?}");
        session.send(r#"{"jsonrpc":"2.0","method":"exit"}"#);
        let (timed_out, output) = session.finish();
        assert!(!timed_out, "{arguments:?}: {output:?}");
        assert_eq!(output.status.code(), Some(0), "{arguments:?}: {output:?}");
    }
}
