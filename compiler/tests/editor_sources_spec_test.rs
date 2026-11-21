//! Saved-disk source selection through the real LSP and explicit-file driver.
//!
//! Fixture graphs and expected paths come from the language module rules.
//! No editor overlay, filename convention, or unrelated main may change the
//! saved program. These process tests do not establish desktop acceptance.

mod editor_sources_support;
#[allow(dead_code)]
mod lsp_support;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

use clc::lsp::json::Json;
use editor_sources_support::*;
use lsp_support::LspClient;

/// Source with no user dependencies; embedded imports are compiler-owned.
const ENTRY: &str =
    "module app;\nimport { println } from std.io;\nfunc main(): i32 { println(\"ok\"); return 0; }\n";

/// Replacement saved entry used to prove that editing imports changes selection.
const CHANGED_ENTRY: &str = "module app.main;\nimport { answer } from changed;\n\
    func main(): i32 { return answer(); }\n";

/// Exported function selected by the replacement entry's import.
const CHANGED_DEP: &str = "module changed;\nexport func answer(): i32 { return 0; }\n";

/// Replacement dependency whose import adds a transitive saved source.
const FORWARDING_DEP: &str = "module app.message;\nimport { text } from changed;\n\
    export func message(): str { return text(); }\n";

/// The transitive dependency returns a distinct, independently known marker.
const CHANGED_LEAF: &str =
    "module changed;\nexport func text(): str { return \"changed-saved-proof\"; }\n";

/// The server explicitly negotiates the new request's version.
#[test]
fn should_advertise_saved_source_selection_version_one() {
    let mut client = LspClient::start();
    let response = client.initialize("host");
    assert_eq!(
        response.get("result")
            .and_then(|value| value.get("capabilities"))
            .and_then(|value| value.get("experimental"))
            .and_then(|value| value.get("clorraineSourceFilesVersion"))
            .and_then(Json::as_i64),
        Some(1),
        "missing saved-source capability: {response:?}"
    );
}

/// The entry alone is returned when only embedded standard-library code is used.
#[test]
fn should_select_an_unopened_entry_without_emitting_embedded_std_files() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", ENTRY);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
}

/// Module identity comes from contents; merged modules and cycles appear once.
#[test]
fn should_include_transitive_cyclic_and_same_module_sources_but_not_other_mains() {
    let fixture = Fixture::new();
    let entry = fixture.write("z-entry.clo",
        "module app;\nimport { pick as chosen } from graph.first;\nfunc main(): i32 { return chosen(); }\n");
    let same = fixture.write("a-entry-part.clo", "module app;\nconst local: i32 = 0;\n");
    let first = fixture.write("not-the-module.clo",
        "module graph.first;\nimport { next } from graph.second;\nexport func pick(): i32 { return next(); }\n");
    let merged = fixture.write("deep/merged.clo",
        "module graph.first;\nexport const extra: i32 = 0;\n");
    let second = fixture.write("elsewhere.clo",
        "module graph.second;\nimport { extra } from graph.first;\nexport func next(): i32 { return extra; }\n");
    fixture.write("unrelated.clo", "module unrelated;\nfunc main(): i32 { return 99; }\n");
    let expected = [same, first, merged, second];
    // Validate the exact graph with the existing CLI before the missing request.
    let mut files = vec![entry.clone()];
    files.extend(expected.iter().cloned());
    exit(&driver(&fixture, "run", "host", &files), 0);
    let mut client = fixture.client();
    let first = select(&mut client, &entry, &fixture.root);
    selected(&first, &entry, &expected);
    assert_eq!(select(&mut client, &entry, &fixture.root).get("result"), first.get("result"));
}

/// Qualified references can discover unopened modules without an import.
#[test]
fn should_select_inventory_known_qualified_modules_without_imports() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo",
        "module app;\nfunc main(): i32 { return graph.math.answer(); }\n");
    let dep = fixture.write("arbitrary.clo",
        "module graph.math;\nexport func answer(): i32 { return graph.leaf.ZERO; }\n");
    let leaf = fixture.write("leaf.clo", "module graph.leaf;\nexport const ZERO: i32 = 0;\n");
    exit(&driver(&fixture, "run", "host", &[entry.clone(), dep.clone(), leaf.clone()]), 0);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep, leaf]);
}

/// Qualified type annotations and constructors also name user modules.
#[test]
fn should_select_modules_named_by_qualified_types_and_struct_literals() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo",
        "module app;\nfunc main(): i32 { let cell: shapes.Cell = shapes.Cell { item: 0 }; return cell.item; }\n");
    let dep = fixture.write("not-shapes.clo",
        "module shapes;\nexport struct Cell { item: i32; }\n");
    exit(&driver(&fixture, "run", "host", &[entry.clone(), dep.clone()]), 0);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep]);
}

/// Creates an inventory module whose spelling is also used as a value binding.
fn shadow_fixture(body: &str) -> (Fixture, PathBuf) {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", &format!(
        "module app;\nstruct Pair {{ item: i32; }}\n{body}\n"));
    fixture.write("value-module.clo", "module value;\nexport const item: i32 = 42;\n");
    (fixture, entry)
}

/// A parameter's field access is not a reference to the equally named module.
#[test]
fn should_distinguish_parameter_fields_from_module_prefixes() {
    let (fixture, entry) = shadow_fixture(
        "func read(value: Pair): i32 { return value.item; }\n\
         func main(): i32 { return read(Pair { item: 0 }); }");
    exit(&driver(&fixture, "run", "host", &[entry.clone()]), 0);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
}

/// Local declarations shadow module names only in their lexical scope.
#[test]
fn should_distinguish_local_fields_from_module_prefixes() {
    let (fixture, entry) = shadow_fixture(
        "func main(): i32 { let value = Pair { item: 0 }; return value.item; }");
    exit(&driver(&fixture, "run", "host", &[entry.clone()]), 0);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
}

/// A module-level value is order-independent and still not a module reference.
#[test]
fn should_distinguish_module_value_fields_from_module_prefixes() {
    let (fixture, entry) = shadow_fixture(
        "func main(): i32 { return value.item; }\nlet value: Pair = Pair { item: 0 };");
    exit(&driver(&fixture, "run", "host", &[entry.clone()]), 0);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
}

/// An imported alias is a binding, even when a different module has that name.
#[test]
fn should_distinguish_import_alias_fields_from_module_prefixes() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo",
        "module app;\nimport { record as value } from records;\n\
         func main(): i32 { return value.item; }\n");
    let dep = fixture.write("records.clo",
        "module records;\nexport struct Pair { item: i32; }\n\
         export let record: Pair = Pair { item: 0 };\n");
    fixture.write("value.clo", "module value;\nexport const item: i32 = 42;\n");
    exit(&driver(&fixture, "run", "host", &[entry.clone(), dep.clone()]), 0);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep]);
}

/// A later local binding cannot erase an earlier qualified module reference.
#[test]
fn should_respect_declaration_order_and_restore_scope_after_inner_blocks() {
    let (fixture, entry) = shadow_fixture(
        "func main(): i32 { let before = value.item; \
         { let value = Pair { item: 0 }; let inside = value.item; } \
         let after = value.item; let value = Pair { item: 0 }; \
         return before + after + value.item - 84; }");
    let dep = fixture.root.join("value-module.clo");
    exit(&driver(&fixture, "run", "host", &[entry.clone(), dep.clone()]), 0);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep]);
}

/// Saved imports are refreshed on every request, without didOpen/didSave.
#[test]
fn should_refresh_saved_imports_and_new_files_between_requests() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", PRINT_ENTRY);
    let dep = fixture.write("old.clo", PRINT_DEP);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep]);
    fixture.write("entry.clo", CHANGED_ENTRY);
    let new_dep = fixture.write("new.clo", CHANGED_DEP);
    let paths = selected(&select(&mut client, &entry, &fixture.root), &entry, &[new_dep]);
    exit(&driver(&fixture, "run", "host", &paths), 0);
}

/// A dependency's saved edits can change the closure while the entry stays fixed.
#[test]
fn should_refresh_imports_changed_inside_a_saved_dependency() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", PRINT_ENTRY);
    let dep = fixture.write("message.clo", PRINT_DEP);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep.clone()]);
    fixture.write("message.clo", FORWARDING_DEP);
    let leaf = fixture.write("new-leaf.clo", CHANGED_LEAF);
    let paths = selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep, leaf]);
    let output = driver(&fixture, "run", "host", &paths);
    exit(&output, 7);
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "changed-saved-proof\n");
    assert_eq!(fs::read_to_string(entry).unwrap(), PRINT_ENTRY);
}

/// Disk selection neither consumes nor persists unsaved imports in open buffers.
#[test]
fn should_ignore_open_and_changed_overlays_without_writing_saved_files() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", PRINT_ENTRY);
    let dep = fixture.write("message.clo", PRINT_DEP);
    let other = fixture.write("overlay.clo", "module overlay;\nexport const item: i32 = 0;\n");
    let entry_bytes = fs::read(&entry).unwrap();
    let dep_bytes = fs::read(&dep).unwrap();
    let mut client = fixture.client();
    let entry_uri = clc::lsp::uri::from_path(&entry);
    client.open(&entry_uri, "module app.main;\nimport { item } from overlay;\nfunc main(): i32 { return item; }\n");
    client.open(&clc::lsp::uri::from_path(&dep), "module app.message;\nimport { item } from absent;\n");
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep.clone()]);
    client.change(&entry_uri, "module app.main;\nimport { item } from absent;\nfunc main(): i32 { return item; }\n");
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep.clone()]);
    assert_eq!(fs::read(&entry).unwrap(), entry_bytes);
    assert_eq!(fs::read(&dep).unwrap(), dep_bytes);
    assert!(other.exists());
}

/// Selection must not replace the independent live diagnostic/navigation overlay.
#[test]
fn should_preserve_live_overlay_diagnostics_and_navigation_after_saved_selection() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", ENTRY);
    let mut client = fixture.client();
    let entry_uri = clc::lsp::uri::from_path(&entry);
    let call = "func main(): i32 { let n: i32 = null; return overlayOnly(); }";
    let overlay = format!("module app;\nfunc overlayOnly(): i32 {{ return 3; }}\n{call}\n");
    client.open(&entry_uri, &overlay);
    let diagnostics = client.diagnostics();
    assert!(!diagnostics.get("diagnostics").and_then(Json::as_array).unwrap().is_empty());
    let character = u32::try_from(call.find("overlayOnly").unwrap()).unwrap();
    let before = client.at("textDocument/definition", &entry_uri, 2, character);
    let location = before.get("result").expect("live overlay definition");
    assert_eq!(location.get("uri").and_then(Json::as_str), Some(entry_uri.as_str()));
    assert_eq!(
        location.get("range").and_then(|range| range.get("start"))
            .and_then(|start| start.get("line")).and_then(Json::as_i64),
        Some(1)
    );
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
    let after = client.at("textDocument/definition", &entry_uri, 2, character);
    assert_eq!(after.get("result"), before.get("result"), "saved selection preserves overlay navigation");
    client.change(&entry_uri, "module app;\nfunc main(): i32 { return 0; }\n");
    let diagnostics = client.diagnostics();
    assert!(diagnostics.get("diagnostics").and_then(Json::as_array).unwrap().is_empty());
    assert_eq!(fs::read_to_string(entry).unwrap(), ENTRY);
}

/// An explicit containing root overrides the initial first workspace root.
#[test]
fn should_route_second_root_selection_as_literal_driver_arguments() {
    let first = Fixture::new();
    first.write("first.clo", ENTRY);
    let fixture = Fixture::new();
    let entry = fixture.write("second root ; $literal/-entry.clo", PRINT_ENTRY);
    let dep = fixture.write("second root ; $literal/dependency &.clo", PRINT_DEP);
    let root = entry.parent().unwrap();
    let mut client = first.client();
    let paths = selected(&select(&mut client, &entry, root), &entry, &[dep]);
    exit(&driver(&fixture, "build", "host", &paths), 0);
    let output = driver(&fixture, "run", "host", &paths);
    exit(&output, 7);
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "saved-source-proof\n");
}

/// No initialized workspace is needed when a standalone file supplies its parent.
#[test]
fn should_route_standalone_selection_through_all_runnable_profiles() {
    let fixture = Fixture::new();
    let entry = fixture.write("-standalone ;.clo", PRINT_ENTRY);
    let dep = fixture.write("dependency $.clo", PRINT_DEP);
    let mut client = LspClient::start();
    client.initialize("host");
    let paths = selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep]);
    for profile in ["host", "hosted-amiga", "hosted-amiga-13"] {
        exit(&driver(&fixture, "build", profile, &paths), 0);
        let output = driver(&fixture, "run", profile, &paths);
        exit(&output, 7);
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "saved-source-proof\n");
    }
}

/// These controls remain independently green before the new request exists.
#[test]
fn should_validate_explicit_host_driver_and_dependency_omission_controls() {
    driver_controls("host");
}

/// Both Amiga profiles are required real compiler/vamos controls, never skips.
#[test]
fn should_validate_explicit_hosted_amiga_driver_and_dependency_omission_controls() {
    driver_controls("hosted-amiga");
}

/// The Kickstart 1.3 profile retains its existing explicit-file command contract.
#[test]
fn should_validate_explicit_hosted_amiga_13_driver_and_dependency_omission_controls() {
    driver_controls("hosted-amiga-13");
}

/// Saved-edit fixtures must compile independently of the not-yet-available request.
#[test]
fn should_validate_changed_import_fixture_outputs_with_explicit_driver_arguments() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", CHANGED_ENTRY);
    let dep = fixture.write("new.clo", CHANGED_DEP);
    exit(&driver(&fixture, "run", "host", &[entry.clone(), dep.clone()]), 0);
    fixture.write("entry.clo", PRINT_ENTRY);
    fixture.write("new.clo", FORWARDING_DEP);
    let leaf = fixture.write("leaf.clo", CHANGED_LEAF);
    let output = driver(&fixture, "run", "host", &[entry, dep, leaf]);
    exit(&output, 7);
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "changed-saved-proof\n");
}

/// Compiler-owned expression and type errors do not prevent file selection.
#[test]
fn should_leave_unresolved_expressions_and_semantic_errors_to_the_compiler() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo",
        "module app;\nfunc main(): i32 { let n: i32 = null; return unknown.field; }\n");
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
}

/// A readable module declaration remains selectable despite a body syntax error.
#[test]
fn should_leave_body_syntax_errors_to_the_compiler() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nfunc main(): i32 { return 0 + ; }\n");
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
}

/// An explicit missing import is a failed selection, not an entry-only success.
#[test]
fn should_reject_a_missing_explicit_user_module() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo",
        "module app;\nimport { missing } from absent.user;\nfunc main(): i32 { return missing(); }\n");
    let mut client = fixture.client();
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// Invalid URI inputs must not be interpreted as paths or accepted partially.
#[test]
fn should_reject_malformed_non_file_and_wrong_typed_uri_parameters() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", ENTRY);
    let mut client = fixture.client();
    for bad in [
        Json::String("not a URI".into()),
        Json::String("untitled:entry.clo".into()),
        Json::String("https://example.test/entry.clo".into()),
        Json::String("file:///bad%GG.clo".into()),
        Json::Number(1),
        Json::Null,
    ] {
        for params in [
            Json::object([("entryUri", bad.clone()), ("rootUri", uri(&fixture.root))]),
            Json::object([("entryUri", uri(&entry)), ("rootUri", bad.clone())]),
        ] {
            rejected(&client.request("clorraine/sourceFiles", params));
        }
    }
    for params in [Json::Null, Json::object([("entryUri", uri(&entry))]), Json::object([("rootUri", uri(&fixture.root))])] {
        rejected(&client.request("clorraine/sourceFiles", params));
    }
}

/// Both path kind and existence matter independently of the initial index.
#[test]
fn should_reject_missing_entry_root_and_wrong_path_kinds() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", ENTRY);
    let mut client = fixture.client();
    for (file, root) in [
        (fixture.root.join("missing.clo"), fixture.root.clone()),
        (entry.clone(), fixture.root.join("missing-root")),
        (fixture.root.clone(), fixture.root.clone()),
        (entry.clone(), entry.clone()),
    ] {
        rejected(&select(&mut client, &file, &root));
    }
}

/// String-prefix containment would incorrectly admit a sibling named root-other.
#[test]
fn should_reject_prefix_sibling_entries_and_parent_traversal_escapes() {
    let fixture = Fixture::new();
    let entry = fixture.write("root-other/entry.clo", ENTRY);
    fs::create_dir(fixture.root.join("root")).unwrap();
    let root = fixture.root.join("root");
    let mut client = fixture.client();
    rejected(&select(&mut client, &entry, &root));
    rejected(&select(&mut client, &root.join("../root-other/entry.clo"), &root));
}

/// Canonical output removes harmless internal parent components, not containment.
#[test]
fn should_return_canonical_paths_for_contained_noncanonical_spellings() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", ENTRY);
    fs::create_dir(fixture.root.join("inner")).unwrap();
    let mut client = fixture.client();
    let root = fixture.root.join("inner/..");
    selected(&select(&mut client, &root.join("entry.clo"), &root), &entry, &[]);
}

/// A selected source symlink cannot import bytes outside the canonical root.
#[test]
fn should_reject_a_selected_file_symlink_escaping_the_root() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    let target = outside.write("message.clo", PRINT_DEP);
    let entry = fixture.write("entry.clo", PRINT_ENTRY);
    symlink(&target, fixture.root.join("message.clo")).unwrap();
    let mut client = fixture.client();
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// Canonical containment also applies to the entry itself.
#[test]
fn should_reject_an_entry_symlink_escaping_the_root() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    let target = outside.write("entry.clo", ENTRY);
    let entry = fixture.root.join("entry.clo");
    symlink(&target, &entry).unwrap();
    let mut client = fixture.client();
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// Skipped symlink directories cannot satisfy an import from outside the root.
#[test]
fn should_not_follow_symlinked_directories_to_satisfy_imports() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    outside.write("message.clo", PRINT_DEP);
    let entry = fixture.write("entry.clo", PRINT_ENTRY);
    symlink(&outside.root, fixture.root.join("linked")).unwrap();
    let mut client = fixture.client();
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// Every generated or hidden directory exclusion also applies to module lookup.
#[test]
fn should_not_resolve_imports_from_generated_or_hidden_directories() {
    for directory in [".git", "build", "target", "node_modules", ".hidden"] {
        let fixture = Fixture::new();
        let entry = fixture.write("entry.clo", PRINT_ENTRY);
        fixture.write(&format!("{directory}/message.clo"), PRINT_DEP);
        let mut client = fixture.client();
        rejected(&select(&mut client, &entry, &fixture.root));
    }
}

/// Removing a saved dependency after indexing must not reuse stale cached bytes.
#[test]
fn should_reject_a_dependency_removed_since_the_previous_request() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", PRINT_ENTRY);
    let dep = fixture.write("message.clo", PRINT_DEP);
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[dep.clone()]);
    fs::remove_file(dep).unwrap();
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// A failed read must not be silently dropped from the requested program.
#[test]
fn should_reject_an_unreadable_selected_dependency() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", PRINT_ENTRY);
    let dep = fixture.write("message.clo", PRINT_DEP);
    let mut client = fixture.client();
    let _permissions = Unreadable::new(&dep);
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// The entry must be read from disk, not replaced by cached initialization text.
#[test]
fn should_reject_an_unreadable_entry() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", ENTRY);
    let mut client = fixture.client();
    let _permissions = Unreadable::new(&entry);
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// A requested root that cannot be enumerated cannot yield a complete index.
#[test]
fn should_reject_an_unreadable_requested_root() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", ENTRY);
    let mut client = fixture.client();
    let _permissions = Unreadable::new(&fixture.root);
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// An unreadable non-excluded subtree makes indexing incomplete, even if unused.
#[test]
fn should_reject_an_incomplete_index_due_to_an_unreadable_subdirectory() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", ENTRY);
    fixture.write("private/other.clo", "module other;\nconst item: i32 = 0;\n");
    let mut client = fixture.client();
    let _permissions = Unreadable::new(&fixture.root.join("private"));
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// Unreadable source bytes must not silently disappear from an otherwise readable tree.
#[test]
fn should_reject_an_incomplete_index_due_to_an_unreadable_source() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", ENTRY);
    let unrelated = fixture.write("other.clo", "module other;\nconst item: i32 = 0;\n");
    let mut client = fixture.client();
    let _permissions = Unreadable::new(&unrelated);
    rejected(&select(&mut client, &entry, &fixture.root));
}
