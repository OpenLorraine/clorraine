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
