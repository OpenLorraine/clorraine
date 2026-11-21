//! Filesystem and public-process helpers for saved-source specifications.
//!
//! Expected dependency lists are supplied by each test, never discovered here.

use std::fs::{self, File, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use clc::lsp::json::Json;

use crate::lsp_support::{error_code, LspClient};

pub mod limits;

/// One owned fixture directory, removed on success and assertion failure.
pub struct Fixture {
    /// Canonical directory used for both source files and process output.
    pub root: PathBuf,
}

impl Fixture {
    /// Creates a unique child of the configured scratch directory.
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::var_os("CODEOPS_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../build/tmp"));
        fs::create_dir_all(&base).expect("create source-test scratch parent");
        let root = base.join(format!(
            "editor-sources-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create exclusively owned source fixture");
        Self {
            root: root.canonicalize().expect("canonical scratch root"),
        }
    }

    /// Writes literal saved source bytes, creating only fixture-owned parents.
    pub fn write(&self, name: &str, source: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir_all(path.parent().expect("source parent")).expect("create source parent");
        fs::write(&path, source).expect("write saved source");
        path
    }

    /// Starts the real server with this root; selection tests need no open buffer.
    pub fn client(&self) -> LspClient {
        let mut client = LspClient::start();
        let response = client.initialize_with_root("host", &self.root);
        assert!(response.get("error").is_none(), "{response:?}");
        client
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("remove owned source fixture");
    }
}

/// File URI for a path, using the existing public URI boundary.
pub fn uri(path: &Path) -> Json {
    Json::String(clc::lsp::uri::from_path(path))
}

/// Calls the approved request without introducing a private selection API.
pub fn select(client: &mut LspClient, entry: &Path, root: &Path) -> Json {
    client.request(
        "clorraine/sourceFiles",
        Json::object([("entryUri", uri(entry)), ("rootUri", uri(root))]),
    )
}

/// Checks JSON-RPC structure without prescribing a new diagnostic code.
fn response_envelope(response: &Json) {
    assert_eq!(
        response.get("jsonrpc").and_then(Json::as_str),
        Some("2.0"),
        "{response:?}"
    );
    assert!(response.get("id").and_then(Json::as_i64).is_some());
}

/// Asserts the exact fixture-authored list, including canonical ordering.
///
/// Only the explicitly supplied expected paths are sorted. This helper does
/// not walk sources, parse declarations, or calculate a dependency closure.
pub fn selected(response: &Json, entry: &Path, rest: &[PathBuf]) -> Vec<PathBuf> {
    response_envelope(response);
    assert!(response.get("error").is_none(), "selection failed: {response:?}");
    let result = response.get("result").expect("selection result");
    let files = result
        .get("files")
        .and_then(Json::as_array)
        .expect("selection result.files must be an array");
    let actual: Vec<PathBuf> = files
        .iter()
        .map(|file| PathBuf::from(file.as_str().expect("each selected file is a path string")))
        .collect();
    let mut tail: Vec<PathBuf> = rest
        .iter()
        .map(|path| path.canonicalize().expect("expected saved file exists"))
        .collect();
    tail.sort();
    let mut expected = vec![entry.canonicalize().expect("expected entry exists")];
    expected.extend(tail);
    assert_eq!(actual, expected, "exact entry-first saved dependency list");
    for path in &actual {
        assert!(path.is_absolute(), "selected path must be absolute");
        assert_eq!(path.canonicalize().unwrap(), *path, "canonical selected path");
    }
    actual
}

/// Requires a real selection failure, not an absent method or a partial result.
///
/// The diagnostic text must contain useful context, but its wording and new
/// numeric code deliberately remain implementation choices.
pub fn rejected(response: &Json) {
    response_envelope(response);
    let code = error_code(response).expect("selection failure has a numeric error code");
    assert_ne!(code, -32601, "unknown method is not a selection diagnostic: {response:?}");
    assert!(response.get("result").is_none(), "no partial result on failure: {response:?}");
    let error = response.get("error").expect("selection error object");
    let message = error
        .get("message")
        .and_then(Json::as_str)
        .expect("selection failure has a message");
    assert!(!message.trim().is_empty(), "selection failure needs an explanation: {response:?}");
    assert!(error.get("files").is_none(), "no partial file list in error");
    if let Some(data) = error.get("data") {
        assert!(data.get("files").is_none(), "no partial file list in error data");
    }
}

/// Temporarily removes all permission bits and restores them during unwinding.
pub struct Unreadable {
    path: PathBuf,
    original: Permissions,
}

impl Unreadable {
    /// Confirms the running user really cannot read this fixture file/directory.
    pub fn new(path: &Path) -> Self {
        let guard = Self {
            path: path.to_owned(),
            original: fs::metadata(path).expect("permission fixture").permissions(),
        };
        fs::set_permissions(path, Permissions::from_mode(0)).expect("deny fixture reads");
        let inaccessible = if path.is_dir() {
            fs::read_dir(path).is_err()
        } else {
            File::open(path).is_err()
        };
        assert!(inaccessible, "required ordinary-user unreadability control");
        guard
    }
}

impl Drop for Unreadable {
    fn drop(&mut self) {
        fs::set_permissions(&self.path, self.original.clone()).expect("restore fixture permissions");
    }
}

/// A valid cross-module program whose print and exit values are independent oracles.
pub const PRINT_ENTRY: &str = "module app.main;\nimport { println } from std.io;\n\
    import { message } from app.message;\n\
    func main(): i32 { println(message()); return 7; }\n";

/// Export is required when the entry imports this declaration across modules.
pub const PRINT_DEP: &str =
    "module app.message;\nexport func message(): str { return \"saved-source-proof\"; }\n";

/// Runs the actual driver with literal absolute source arguments and a deadline.
///
/// Output goes to owned files, not unbounded pipes. `timeout` is a required SDK
/// tool; a missing compiler or runner is a failure, never a skipped test.
/// These fixtures use the entry's parent as their containing action root.
pub fn driver(fixture: &Fixture, action: &str, profile: &str, files: &[PathBuf]) -> Output {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let stdout = fixture.root.join(format!("driver-{sequence}.stdout"));
    let stderr = fixture.root.join(format!("driver-{sequence}.stderr"));
    let status = Command::new("timeout")
        .args(["--kill-after=2s", "60s", env!("CARGO_BIN_EXE_clorraine")])
        .args([action, "--profile", profile])
        .args(files)
        .current_dir(files[0].parent().expect("absolute entry parent"))
        .stdin(Stdio::null())
        .stdout(File::create(&stdout).expect("driver stdout log"))
        .stderr(File::create(&stderr).expect("driver stderr log"))
        .status()
        .expect("start bounded real driver");
    assert_ne!(status.code(), Some(124), "real driver exceeded deadline");
    Output {
        status,
        stdout: fs::read(stdout).expect("read driver stdout"),
        stderr: fs::read(stderr).expect("read driver stderr"),
    }
}

/// Checks real build/run results and includes both captured streams on failure.
pub fn exit(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Checks explicit-file positive and omission controls before any new LSP call.
pub fn driver_controls(profile: &str) {
    let fixture = Fixture::new();
    let entry = fixture.write("second root ; $literal/-entry.clo", PRINT_ENTRY);
    let dep = fixture.write("second root ; $literal/odd dependency &.clo", PRINT_DEP);
    let files = vec![entry.clone(), dep];
    exit(&driver(&fixture, "build", profile, &files), 0);
    let output = driver(&fixture, "run", profile, &files);
    exit(&output, 7);
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "saved-source-proof\n");
    let omitted = driver(&fixture, "build", profile, &[entry]);
    assert!(!omitted.status.success(), "CLI must not discover an omitted dependency");
    assert!(
        String::from_utf8_lossy(&omitted.stderr).contains("app.message"),
        "omission control must identify the missing user module"
    );
}
