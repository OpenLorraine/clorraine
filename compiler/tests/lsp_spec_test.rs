//! Specification tests for the language-server protocol core.
//!
//! These cases derive from the LSP requirements: initialize and lifecycle,
//! framing errors, unknown methods, and cancellation. The feature surface
//! (diagnostics, hover, goto, completion, symbols) has its own cases in the
//! same suite later in the plan.

mod lsp_support;

use clc::lsp::json::Json;
use lsp_support::{error_code, LspClient};

/// Builds an empty params object.
fn empty() -> Json {
    Json::Object(Default::default())
}

#[test]
fn should_initialize_with_the_minimum_capabilities() {
    let mut client = LspClient::start();
    let response = client.initialize("host");
    let result = response.get("result").expect("initialize result");
    let capabilities = result.get("capabilities").expect("capabilities");
    assert_eq!(
        capabilities.get("positionEncoding").and_then(Json::as_str),
        Some("utf-16")
    );
    assert_eq!(
        capabilities.get("textDocumentSync").and_then(Json::as_i64),
        Some(1)
    );
    assert_eq!(
        capabilities.get("hoverProvider"),
        Some(&Json::Bool(true))
    );
    assert_eq!(
        capabilities.get("definitionProvider"),
        Some(&Json::Bool(true))
    );
    assert!(capabilities.get("completionProvider").is_some());
    assert_eq!(
        capabilities.get("documentSymbolProvider"),
        Some(&Json::Bool(true))
    );
    assert_eq!(
        result
            .get("serverInfo")
            .and_then(|info| info.get("name"))
            .and_then(Json::as_str),
        Some("clorraine-lsp")
    );
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_exit_one_without_shutdown() {
    let mut client = LspClient::start();
    client.initialize("host");
    client.send_text(
        &Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("method", Json::String("exit".to_owned())),
        ])
        .render(),
    );
    assert_eq!(client.wait_exit(), 1);
}

#[test]
fn should_reject_a_request_before_initialize() {
    let mut client = LspClient::start();
    let response = client.request("textDocument/hover", empty());
    assert_eq!(error_code(&response), Some(-32002));
    let initialized = client.initialize("host");
    assert!(initialized.get("result").is_some());
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_report_malformed_json_and_continue() {
    let mut client = LspClient::start();
    client.send_text("{not json");
    let error = client.message();
    assert_eq!(error.get("id"), Some(&Json::Null));
    assert_eq!(error_code(&error), Some(-32700));
    let initialized = client.initialize("host");
    assert!(initialized.get("result").is_some());
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_report_an_unknown_request_method() {
    let mut client = LspClient::start();
    client.initialize("host");
    let response = client.request("workspace/nothing", empty());
    assert_eq!(error_code(&response), Some(-32601));
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_ignore_an_unknown_notification() {
    let mut client = LspClient::start();
    client.send_text(
        &Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("method", Json::String("workspace/nothing".to_owned())),
            ("params", empty()),
        ])
        .render(),
    );
    let initialized = client.initialize("host");
    assert!(initialized.get("result").is_some());
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_ignore_a_cancellation_for_an_unknown_id() {
    let mut client = LspClient::start();
    client.send_text(
        &Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("method", Json::String("$/cancelRequest".to_owned())),
            (
                "params",
                Json::object([("id", Json::Number(999))]),
            ),
        ])
        .render(),
    );
    let initialized = client.initialize("host");
    assert!(initialized.get("result").is_some());
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_cancel_a_request_before_it_starts() {
    let mut client = LspClient::start();
    client.send_text(
        &Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("method", Json::String("$/cancelRequest".to_owned())),
            ("params", Json::object([("id", Json::Number(7))])),
        ])
        .render(),
    );
    client.request_with_id(7, "textDocument/hover", empty());
    let response = client.await_response(7);
    assert_eq!(error_code(&response), Some(-32800));
    client.initialize("host");
    assert_eq!(client.shutdown_and_exit(), 0);
}

// ---- Feature surface: diagnostics and navigation ----

use std::path::{Path, PathBuf};

/// A fixture directory under `compiler/tests/fixtures/lsp`.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/lsp")
        .join(name)
}

/// The `file:` URI of a fixture file.
fn fixture_uri(name: &str, file: &str) -> String {
    clc::lsp::uri::from_path(&fixture(name).join(file))
}

/// The document text of a fixture file.
fn fixture_text(name: &str, file: &str) -> String {
    std::fs::read_to_string(fixture(name).join(file)).expect("fixture")
}

/// Extracts the diagnostics array.
fn diagnostics_array(params: &Json) -> &[Json] {
    params
        .get("diagnostics")
        .and_then(Json::as_array)
        .expect("diagnostics array")
}

#[test]
fn should_publish_a_diagnostic_when_a_document_opens() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("errors"));
    let uri = fixture_uri("errors", "bad.clo");
    client.open(&uri, &fixture_text("errors", "bad.clo"));
    let params = client.diagnostics();
    assert_eq!(params.get("uri").and_then(Json::as_str), Some(uri.as_str()));
    let diagnostics = diagnostics_array(&params);
    assert_eq!(diagnostics.len(), 1, "one error, got {diagnostics:?}");
    let code = diagnostics[0]
        .get("code")
        .and_then(Json::as_str)
        .expect("code");
    assert!(code.starts_with('E'), "expected an error code, got {code}");
    assert_eq!(
        diagnostics[0]
            .get("source")
            .and_then(Json::as_str),
        Some("clorraine")
    );
    assert_eq!(
        diagnostics[0]
            .get("severity")
            .and_then(Json::as_i64),
        Some(1)
    );
    let start = diagnostics[0]
        .get("range")
        .and_then(|range| range.get("start"))
        .expect("range start");
    assert_eq!(start.get("line").and_then(Json::as_i64), Some(3));
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_clear_diagnostics_when_the_buffer_is_fixed() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("errors"));
    let uri = fixture_uri("errors", "bad.clo");
    client.open(&uri, &fixture_text("errors", "bad.clo"));
    assert_eq!(diagnostics_array(&client.diagnostics()).len(), 1);
    client.change(
        &uri,
        "module app.bad;\nfunc main(): i32 {\n    return 0;\n}\n",
    );
    assert_eq!(diagnostics_array(&client.diagnostics()).len(), 0);
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_clear_diagnostics_when_the_document_closes() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("errors"));
    let uri = fixture_uri("errors", "bad.clo");
    client.open(&uri, &fixture_text("errors", "bad.clo"));
    assert_eq!(diagnostics_array(&client.diagnostics()).len(), 1);
    client.close(&uri);
    assert_eq!(diagnostics_array(&client.diagnostics()).len(), 0);
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_return_null_for_a_missing_document() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("workspace"));
    let missing = clc::lsp::uri::from_path(&fixture("workspace").join("nope.clo"));
    let hover = client.at("textDocument/hover", &missing, 0, 0);
    assert_eq!(hover.get("result"), Some(&Json::Null));
    let definition = client.at("textDocument/definition", &missing, 0, 0);
    assert_eq!(definition.get("result"), Some(&Json::Null));
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_not_mix_independent_programs() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("workspace"));
    let main = fixture_uri("workspace", "main.clo");
    client.open(&main, &fixture_text("workspace", "main.clo"));
    let params = client.diagnostics();
    assert!(
        diagnostics_array(&params).is_empty(),
        "unexpected diagnostics: {params:?}"
    );
    let other = fixture_uri("workspace", "other.clo");
    client.open(&other, &fixture_text("workspace", "other.clo"));
    let params = client.diagnostics();
    assert!(
        diagnostics_array(&params).is_empty(),
        "unexpected diagnostics: {params:?}"
    );
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_use_the_selected_profile() {
    let mut host = LspClient::start();
    host.initialize_with_root("host", &fixture("amiga"));
    let uri = fixture_uri("amiga", "amiga.clo");
    host.open(&uri, &fixture_text("amiga", "amiga.clo"));
    let host_diagnostics = diagnostics_array(&host.diagnostics()).to_vec();
    assert!(
        host_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.get("code").and_then(Json::as_str) == Some("E11200")),
        "host profile must reject std.amiga: {host_diagnostics:?}"
    );
    assert_eq!(host.shutdown_and_exit(), 0);

    let mut amiga = LspClient::start();
    amiga.initialize_with_root("hosted-amiga", &fixture("amiga"));
    amiga.open(&uri, &fixture_text("amiga", "amiga.clo"));
    let amiga_diagnostics = diagnostics_array(&amiga.diagnostics()).to_vec();
    assert!(
        amiga_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.get("code").and_then(Json::as_str) != Some("E11200")),
        "hosted-amiga must accept std.amiga: {amiga_diagnostics:?}"
    );
    assert_eq!(amiga.shutdown_and_exit(), 0);
}

#[test]
fn should_serve_an_untitled_buffer() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("workspace"));
    let uri = "untitled:Untitled-1";
    client.open(
        uri,
        "module app.new;\nfunc main(): i32 {\n    let x: i32 = null;\n    return x;\n}\n",
    );
    let params = client.diagnostics();
    assert_eq!(params.get("uri").and_then(Json::as_str), Some(uri));
    assert_eq!(diagnostics_array(&params).len(), 1);
    let symbols = client.symbols(uri);
    let names: Vec<&str> = symbols
        .get("result")
        .and_then(Json::as_array)
        .expect("symbols")
        .iter()
        .filter_map(|symbol| symbol.get("name").and_then(Json::as_str))
        .collect();
    assert_eq!(names, vec!["main"]);
    let hover = client.at("textDocument/hover", uri, 1, 5);
    assert!(hover.get("result").is_some());
    let completion = client.at("textDocument/completion", uri, 1, 0);
    let labels: Vec<&str> = completion
        .get("result")
        .and_then(Json::as_array)
        .expect("completion")
        .iter()
        .filter_map(|item| item.get("label").and_then(Json::as_str))
        .collect();
    assert!(labels.contains(&"main"), "completion was {labels:?}");
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_hover_a_documented_function() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("workspace"));
    let main = fixture_uri("workspace", "main.clo");
    client.open(&main, &fixture_text("workspace", "main.clo"));
    let hover = client.at("textDocument/hover", &main, 6, 19);
    let value = hover
        .get("result")
        .and_then(|result| result.get("contents"))
        .and_then(|contents| contents.get("value"))
        .and_then(Json::as_str)
        .expect("hover value");
    assert!(
        value.contains("func add(a: i32, b: i32): i32"),
        "hover was {value:?}"
    );
    assert!(value.contains("function"), "hover was {value:?}");
    assert!(value.contains("app.lib"), "hover was {value:?}");
    let range = hover
        .get("result")
        .and_then(|result| result.get("range"))
        .expect("hover range");
    assert_eq!(
        range.get("start").and_then(|start| start.get("line")).and_then(Json::as_i64),
        Some(6)
    );
    assert_eq!(
        range.get("start").and_then(|start| start.get("character")).and_then(Json::as_i64),
        Some(19)
    );
    assert_eq!(
        range.get("end").and_then(|end| end.get("character")).and_then(Json::as_i64),
        Some(22)
    );
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_go_to_a_local_definition() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("workspace"));
    let main = fixture_uri("workspace", "main.clo");
    client.open(&main, &fixture_text("workspace", "main.clo"));
    let definition = client.at("textDocument/definition", &main, 7, 11);
    let range = definition
        .get("result")
        .and_then(|result| result.get("range"))
        .expect("definition range");
    assert_eq!(
        range.get("start").and_then(|start| start.get("line")).and_then(Json::as_i64),
        Some(6)
    );
    assert_eq!(
        range.get("start").and_then(|start| start.get("character")).and_then(Json::as_i64),
        Some(8)
    );
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_go_to_an_imported_definition() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("workspace"));
    let main = fixture_uri("workspace", "main.clo");
    client.open(&main, &fixture_text("workspace", "main.clo"));
    let definition = client.at("textDocument/definition", &main, 6, 19);
    let result = definition.get("result").expect("definition result");
    let uri = result.get("uri").and_then(Json::as_str).expect("uri");
    assert!(uri.ends_with("/lib.clo"), "definition uri was {uri}");
    let range = result.get("range").expect("range");
    assert_eq!(
        range.get("start").and_then(|start| start.get("line")).and_then(Json::as_i64),
        Some(3)
    );
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_complete_keywords_types_declarations_and_modules() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("workspace"));
    let main = fixture_uri("workspace", "main.clo");
    client.open(&main, &fixture_text("workspace", "main.clo"));
    let completion = client.at("textDocument/completion", &main, 4, 0);
    let labels: Vec<&str> = completion
        .get("result")
        .and_then(Json::as_array)
        .expect("completion")
        .iter()
        .filter_map(|item| item.get("label").and_then(Json::as_str))
        .collect();
    for expected in ["func", "i32", "add", "app.lib"] {
        assert!(labels.contains(&expected), "missing {expected} in {labels:?}");
    }
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted, "completion must be sorted");
    let mut unique = labels.clone();
    unique.dedup();
    assert_eq!(labels, unique, "completion must be unique");
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_list_document_symbols_in_source_order() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("workspace"));
    let lib = fixture_uri("workspace", "lib.clo");
    let symbols = client.symbols(&lib);
    let entries = symbols
        .get("result")
        .and_then(Json::as_array)
        .expect("symbols");
    let names: Vec<&str> = entries
        .iter()
        .filter_map(|symbol| symbol.get("name").and_then(Json::as_str))
        .collect();
    assert_eq!(names, vec!["add", "Point", "ORIGIN_X"]);
    let kinds: Vec<i64> = entries
        .iter()
        .filter_map(|symbol| symbol.get("kind").and_then(Json::as_i64))
        .collect();
    assert_eq!(kinds, vec![12, 23, 14]);
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_not_crash_on_an_embedded_module_reference() {
    let mut client = LspClient::start();
    client.initialize_with_root("hosted-amiga", &fixture("amiga"));
    let uri = fixture_uri("amiga", "amiga.clo");
    client.open(&uri, &fixture_text("amiga", "amiga.clo"));
    let _ = client.diagnostics();
    let hover = client.at("textDocument/hover", &uri, 2, 9);
    let definition = client.at("textDocument/definition", &uri, 2, 9);
    assert_eq!(hover.get("result"), Some(&Json::Null));
    assert_eq!(definition.get("result"), Some(&Json::Null));
    let symbols = client.symbols(&uri);
    assert!(symbols.get("result").is_some());
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_reject_requests_after_shutdown() {
    let mut client = LspClient::start();
    client.initialize("host");
    let shutdown = client.request("shutdown", Json::Null);
    assert_eq!(shutdown.get("result"), Some(&Json::Null));
    let hover = client.request("textDocument/hover", empty());
    assert_eq!(error_code(&hover), Some(-32600));
    let again = client.request("shutdown", Json::Null);
    assert_eq!(error_code(&again), Some(-32600));
    client.send_text(
        &Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("method", Json::String("exit".to_owned())),
        ])
        .render(),
    );
    assert_eq!(client.wait_exit(), 0);
}

#[test]
fn should_ignore_document_notifications_before_initialize() {
    let mut client = LspClient::start();
    client.open(
        "file:///tmp/pre-init.clo",
        "module app.pre;\nfunc main(): i32 { let x: i32 = null; return x; }\n",
    );
    let initialized = client.initialize("host");
    assert!(initialized.get("result").is_some());
    assert_eq!(
        client.buffered(),
        0,
        "a pre-initialize didOpen must not publish diagnostics"
    );
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_not_answer_an_unknown_notification() {
    let mut client = LspClient::start();
    client.send_text(
        &Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("method", Json::String("workspace/nothing".to_owned())),
            ("params", empty()),
        ])
        .render(),
    );
    let initialized = client.initialize("host");
    assert!(initialized.get("result").is_some());
    assert_eq!(client.buffered(), 0, "a notification must have no response");
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_reject_an_oversized_body_without_reading_it() {
    use clc::lsp::protocol::{FrameError, FrameReader, MAX_BODY};
    use std::io::Cursor;
    let header = format!("Content-Length: {}\r\n\r\n", MAX_BODY + 1);
    let mut reader = FrameReader::new(Cursor::new(header.into_bytes()));
    assert!(matches!(reader.read(), Err(FrameError::TooLarge)));
}

#[test]
fn should_resolve_a_local_before_a_struct_in_another_file() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("workspace"));
    let main = fixture_uri("workspace", "main.clo");
    client.open(&main, &fixture_text("workspace", "main.clo"));
    let hover = client.at("textDocument/hover", &main, 6, 28);
    let value = hover
        .get("result")
        .and_then(|result| result.get("contents"))
        .and_then(|contents| contents.get("value"))
        .and_then(Json::as_str)
        .expect("hover value");
    assert!(value.contains("let p"), "hover was {value:?}");
    let definition = client.at("textDocument/definition", &main, 6, 28);
    let result = definition.get("result").expect("definition");
    let range = result.get("range").expect("range");
    assert_eq!(
        range.get("start").and_then(|start| start.get("line")).and_then(Json::as_i64),
        Some(5)
    );
    assert_eq!(
        range.get("start").and_then(|start| start.get("character")).and_then(Json::as_i64),
        Some(8)
    );
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_complete_a_declaration_from_a_sibling_file_of_the_same_module() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("shared"));
    let a = fixture_uri("shared", "a.clo");
    client.open(&a, &fixture_text("shared", "a.clo"));
    let completion = client.at("textDocument/completion", &a, 5, 0);
    let labels: Vec<&str> = completion
        .get("result")
        .and_then(Json::as_array)
        .expect("completion")
        .iter()
        .filter_map(|item| item.get("label").and_then(Json::as_str))
        .collect();
    assert!(labels.contains(&"helper"), "completion was {labels:?}");
    assert!(labels.contains(&"freedBy"), "completion was {labels:?}");
    assert!(labels.contains(&"static"), "completion was {labels:?}");
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_serve_partial_symbols_for_a_syntax_error_file() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("errors"));
    let syntax = fixture_uri("errors", "syntax.clo");
    client.open(&syntax, &fixture_text("errors", "syntax.clo"));
    let _ = client.diagnostics();
    let symbols = client.symbols(&syntax);
    let names: Vec<&str> = symbols
        .get("result")
        .and_then(Json::as_array)
        .expect("symbols")
        .iter()
        .filter_map(|symbol| symbol.get("name").and_then(Json::as_str))
        .collect();
    assert!(names.contains(&"good"), "symbols were {names:?}");
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_return_null_symbols_for_a_missing_document() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("workspace"));
    let missing = clc::lsp::uri::from_path(&fixture("workspace").join("nope.clo"));
    let symbols = client.symbols(&missing);
    assert_eq!(symbols.get("result"), Some(&Json::Null));
    assert_eq!(client.shutdown_and_exit(), 0);
}


#[test]
fn should_publish_diagnostics_on_save() {
    let mut client = LspClient::start();
    client.initialize_with_root("host", &fixture("errors"));
    let uri = fixture_uri("errors", "bad.clo");
    client.open(&uri, &fixture_text("errors", "bad.clo"));
    assert_eq!(diagnostics_array(&client.diagnostics()).len(), 1);
    client.save(
        &uri,
        "module app.bad;\nfunc main(): i32 {\n    return 0;\n}\n",
    );
    assert_eq!(diagnostics_array(&client.diagnostics()).len(), 0);
    assert_eq!(client.shutdown_and_exit(), 0);
}

#[test]
fn should_ignore_a_cancel_for_a_completed_request() {
    let mut client = LspClient::start();
    client.initialize("host");
    client.request_with_id(5, "textDocument/hover", empty());
    let first = client.await_response(5);
    assert!(first.get("error").is_none());
    client.send_text(
        &Json::object([
            ("jsonrpc", Json::String("2.0".to_owned())),
            ("method", Json::String("$/cancelRequest".to_owned())),
            ("params", Json::object([("id", Json::Number(5))])),
        ])
        .render(),
    );
    client.request_with_id(5, "textDocument/hover", empty());
    let second = client.await_response(5);
    assert!(
        second.get("error").is_none(),
        "a late cancel must not cancel a reused id: {second:?}"
    );
    assert_eq!(client.shutdown_and_exit(), 0);
}
