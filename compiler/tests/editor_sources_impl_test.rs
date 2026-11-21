//! Saved-source traversal boundaries and lexical-metadata implementation controls.
//! Real compiler builds validate binding examples before their saved selections
//! are asserted. Existing process helpers and immutable limit controls are reused.

#[allow(dead_code)]
mod editor_sources_support;
#[allow(dead_code)]
mod lsp_support;

use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use editor_sources_support::{driver, exit, rejected, select, selected, Fixture};

/// Check a valid binding example against an inventory containing its head name.
fn binding_case(declarations: &str, body: &str, dependency_needed: bool) {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", &format!(
        "module app;\nstruct Pair {{ item: i32; }}\n{declarations}\n\
         func main(): i32 {{ {body} }}\n"));
    let dependency = fixture.write("value.clo",
        "module value;\nexport const item: i32 = 42;\n\
         export func items(): i32[1] { return [0]; }\n");
    let expected = if dependency_needed { vec![dependency] } else { vec![] };
    let mut files = vec![entry.clone()];
    files.extend(expected.iter().cloned());
    exit(&driver(&fixture, "build", "host", &files), 0);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &expected);
}

/// A local does not bind its name while its own initializer is evaluated.
#[test]
fn should_select_module_references_in_a_local_initializer() {
    binding_case("", "let value = value.item; return value - 42;", true);
}

/// Const initializers have the same visibility boundary as ordinary local lets.
#[test]
fn should_select_module_references_in_a_const_initializer() {
    binding_case("", "const value: i32 = value.item; return value - 42;", true);
}

/// A counted-loop initializer precedes the loop-local binding's visibility.
#[test]
fn should_select_module_references_in_a_counted_loop_initializer() {
    binding_case("", "for (let value: i32 = value.item; value < 43; value += 1) {} return 0;", true);
}

/// The counted-loop variable is already visible in its condition and step.
#[test]
fn should_keep_counted_loop_field_accesses_out_of_the_module_closure() {
    binding_case("", "for (let value = Pair { item: 0 }; value.item < 1; value.item += 1) {} return 0;", false);
}

/// An element-loop variable is visible in its body, not in the iterable.
#[test]
fn should_select_a_module_named_by_an_element_loop_iterable() {
    binding_case("", "let total: i32 = 0; for (value in value.items()) { total += value; } return total;", true);
}

/// Leaving an element loop restores references to an equally named module.
#[test]
fn should_restore_module_visibility_after_an_element_loop() {
    binding_case("", "let rows: Pair[1] = [Pair { item: 0 }]; \
        for (value in rows) { let inside = value.item; } return value.item - 42;", true);
}

/// Lambda parameter metadata prevents a value field from selecting a module.
#[test]
fn should_keep_lambda_parameters_local_to_the_lambda_body() {
    binding_case("", "let pick: func(Pair): i32 = (value: Pair) => value.item; \
        return pick(Pair { item: 0 });", false);
}

/// Lambda parameters cannot hide a subsequent outer module reference.
#[test]
fn should_restore_module_visibility_after_a_lambda_expression() {
    binding_case("", "let pick: func(Pair): i32 = (value: Pair) => value.item; \
        return pick(Pair { item: 0 }) + value.item - 42;", true);
}

/// Global initializer lambdas need the same local metadata as function-local ones.
#[test]
fn should_index_parameters_in_a_global_lambda_initializer() {
    binding_case("let pick: func(Pair): i32 = (value: Pair) => value.item;",
        "return pick(Pair { item: 0 });", false);
}

/// Exclusions compare full directory names, not prefixes of allowed names.
#[test]
fn should_index_ordinary_directories_whose_names_start_with_excluded_names() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nfunc main(): i32 { return 0; }\n");
    let parts: Vec<PathBuf> = ["builder", "targeted", "node_modules_extra"].iter()
        .enumerate().map(|(index, name)| fixture.write(
            &format!("{name}/part.clo"),
            &format!("module app;\nconst value{index}: i32 = {index};\n")
        )).collect();
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &parts);
}

/// URI percent encoding must preserve literal punctuation and Unicode filenames.
#[test]
fn should_preserve_literal_percent_hash_and_unicode_source_names() {
    let fixture = Fixture::new();
    let entry = fixture.write("main % # λ.clo",
        "module app;\nfunc main(): i32 { return value.item - 42; }\n");
    let dependency = fixture.write("é % #.clo", "module value;\nexport const item: i32 = 42;\n");
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dependency]);
}

/// An unrepresentable path must fail rather than become a lossy CLI argument.
#[test]
fn should_reject_non_utf8_source_paths_without_a_partial_selection() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nfunc main(): i32 { return 0; }\n");
    fs::write(fixture.root.join(OsString::from_vec(b"invalid-\xff.clo".to_vec())),
        "module spare;\nconst item: i32 = 0;\n").unwrap();
    let mut client = fixture.client();
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// Overflow is per request; removing one excess indexed file permits a retry.
#[test]
fn should_recover_a_fresh_index_after_removing_the_overflow_file() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nfunc main(): i32 { return 0; }\n");
    let mut extra = PathBuf::new();
    for index in 0..2000 {
        extra = fixture.write(&format!("inventory/{index:04}.clo"),
            &format!("module spare.m{index};\nconst item: i32 = 0;\n"));
    }
    let mut client = fixture.client();
    rejected(&select(&mut client, &entry, &fixture.root));
    fs::remove_file(extra).unwrap();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
}

/// Merged-module budget recovery must also discard the failed request's state.
#[test]
fn should_recover_selection_after_removing_the_257th_merged_file() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nfunc main(): i32 { return 0; }\n");
    let mut parts = Vec::new();
    for index in 0..256 {
        parts.push(fixture.write(&format!("part-{index:04}.clo"),
            &format!("module app;\nconst item{index}: i32 = 0;\n")));
    }
    let mut client = fixture.client();
    rejected(&select(&mut client, &entry, &fixture.root));
    fs::remove_file(parts.pop().unwrap()).unwrap();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &parts);
}

/// Saved selections remain independent of open overlays before and after disk edits.
#[test]
fn should_refresh_disk_sources_without_consuming_or_writing_an_open_overlay() {
    let fixture = Fixture::new();
    let original = "module app;\nfunc main(): i32 { return 0; }\n";
    let changed = "module app;\nimport { item } from value;\nfunc main(): i32 { return item; }\n";
    let entry = fixture.write("entry.clo", original);
    let dependency = fixture.write("value.clo", "module value;\nexport const item: i32 = 42;\n");
    let mut client = fixture.client();
    client.open(&clc::lsp::uri::from_path(&entry), changed);
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
    assert_eq!(fs::read_to_string(&entry).unwrap(), original);
    fs::write(&entry, changed).unwrap();
    client.change(&clc::lsp::uri::from_path(&entry), original);
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dependency]);
    assert_eq!(fs::read_to_string(&entry).unwrap(), changed);
}
