//! Saved-source selection must distinguish complete value chains from module paths.
//! Every fixture first passes a real explicit-file host build and run. Selection
//! is then checked through framed LSP requests, never private compiler helpers.

#[allow(dead_code)]
mod editor_sources_support;
#[allow(dead_code)]
mod lsp_support;

use editor_sources_support::{driver, exit, select, selected, Fixture};

/// A coincidental module exports the same final name as the tested expression.
const MATH: &str = "module graph.math;\nexport func answer(): i32 { return 0; }\n";

/// Prove the intended language program before asserting its exact saved closure.
///
/// `dependencies` is fixture-authored; this helper does not discover references.
fn check(entry_source: &str, dependencies: &[(&str, &str)], module_needed: bool) {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", entry_source);
    let math = fixture.write("coincidental.clo", MATH);
    let mut expected: Vec<_> = dependencies
        .iter()
        .map(|(name, source)| fixture.write(name, source))
        .collect();
    if module_needed {
        expected.push(math);
    }
    let mut explicit = vec![entry.clone()];
    explicit.extend(expected.iter().cloned());
    exit(&driver(&fixture, "build", "host", &explicit), 0);
    exit(&driver(&fixture, "run", "host", &explicit), 0);
    let mut client = fixture.client();
    let paths = selected(&select(&mut client, &entry, &fixture.root), &entry, &expected);
    exit(&driver(&fixture, "build", "host", &paths), 0);
    exit(&driver(&fixture, "run", "host", &paths), 0);
}

/// Nonlocal scalar bindings do not veto a longer qualified module path.
#[test]
fn should_select_a_qualified_module_despite_a_same_headed_global_scalar() {
    check("module app;\nlet graph: i32 = 7;\n\
        func main(): i32 { return graph.math.answer(); }\n", &[], true);
}

/// A function declaration is not proof of a full value-member chain.
#[test]
fn should_select_a_qualified_module_despite_a_same_headed_function() {
    check("module app;\nfunc graph(): i32 { return 7; }\n\
        func main(): i32 { return graph.math.answer(); }\n", &[], true);
}

/// A type declaration alone cannot stand for a value-member chain.
#[test]
fn should_select_a_qualified_module_despite_a_same_headed_struct() {
    check("module app;\nstruct graph { item: i32; }\n\
        func main(): i32 { return graph.math.answer(); }\n", &[], true);
}

/// An imported scalar alias retains its real import and the qualified module.
#[test]
fn should_select_a_qualified_module_despite_a_same_headed_import_alias() {
    check("module app;\nimport { scalar as graph } from records;\n\
        func main(): i32 { return graph.math.answer(); }\n",
        &[("records.clo", "module records;\nexport const scalar: i32 = 7;\n")], true);
}

/// Module merging does not turn a nonlocal scalar into a lexical local.
#[test]
fn should_select_a_qualified_module_when_the_scalar_is_in_a_merged_file() {
    check("module app;\nfunc main(): i32 { return graph.math.answer(); }\n",
        &[("merged.clo", "module app;\nlet graph: i32 = 7;\n")], true);
}

/// A static view is another nonlocal value, not a complete member chain.
#[test]
fn should_select_a_qualified_module_despite_a_same_headed_static_view() {
    check("module app;\nlet graph: static str = \"not a record\";\n\
        func main(): i32 { return graph.math.answer(); }\n", &[], true);
}

/// Finding the first field is insufficient when the next member cannot exist.
#[test]
fn should_select_a_module_when_only_the_first_value_field_exists() {
    check("module app;\nstruct Pair { math: i32; }\n\
        let graph: Pair = Pair { math: 7 };\n\
        func main(): i32 { return graph.math.answer(); }\n", &[], true);
}

/// The incomplete-chain rule also applies through an imported value alias.
#[test]
fn should_select_a_module_when_an_imported_alias_has_only_the_first_field() {
    check("module app;\nimport { record as graph } from records;\n\
        func main(): i32 { return graph.math.answer(); }\n",
        &[("records.clo", "module records;\nexport struct Pair { math: i32; }\n\
            export let record: Pair = Pair { math: 7 };\n")], true);
}

/// Complete nested function fields prefer the smaller field-based closure.
#[test]
fn should_omit_a_coincidental_module_for_a_complete_nested_function_field() {
    check("module app;\nstruct Leaf { answer: func(): i32; }\n\
        struct Pair { math: Leaf; }\nfunc zero(): i32 { return 0; }\n\
        let graph: Pair = Pair { math: Leaf { answer: zero } };\n\
        func main(): i32 { return graph.math.answer(); }\n", &[], false);
}

/// A chain ending in a scalar is equally genuine; call syntax is not required.
#[test]
fn should_omit_a_coincidental_module_for_a_complete_nested_scalar_field() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\n\
        struct Leaf { answer: i32; }\nstruct Pair { math: Leaf; }\n\
        let graph: Pair = Pair { math: Leaf { answer: 0 } };\n\
        func main(): i32 { return graph.math.answer; }\n");
    fixture.write("coincidental.clo", "module graph.math;\nexport const answer: i32 = 42;\n");
    exit(&driver(&fixture, "run", "host", &[entry.clone()]), 0);
    let mut client = fixture.client();
    let paths = selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
    exit(&driver(&fixture, "build", "host", &paths), 0);
    exit(&driver(&fixture, "run", "host", &paths), 0);
}

/// Parsed generic field substitution can establish every member of the chain.
#[test]
fn should_omit_a_coincidental_module_for_a_complete_generic_field_chain() {
    check("module app;\nstruct Leaf { answer: func(): i32; }\n\
        struct Pair<T> { math: T; }\nfunc zero(): i32 { return 0; }\n\
        let graph: Pair<Leaf> = Pair<Leaf> { math: Leaf { answer: zero } };\n\
        func main(): i32 { return graph.math.answer(); }\n", &[], false);
}

/// A longer chain cannot be classified from its head or its length alone.
#[test]
fn should_omit_a_coincidental_module_for_a_deeper_complete_member_chain() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\n\
        struct Leaf { answer: func(): i32; }\nstruct Middle { leaf: Leaf; }\n\
        struct Pair { math: Middle; }\nfunc zero(): i32 { return 0; }\n\
        let graph: Pair = Pair { math: Middle { leaf: Leaf { answer: zero } } };\n\
        func main(): i32 { return graph.math.leaf.answer(); }\n");
    fixture.write("coincidental.clo", "module graph.math.leaf;\n\
        export func answer(): i32 { return 42; }\n");
    exit(&driver(&fixture, "build", "host", &[entry.clone()]), 0);
    exit(&driver(&fixture, "run", "host", &[entry.clone()]), 0);
    let mut client = fixture.client();
    let paths = selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
    exit(&driver(&fixture, "run", "host", &paths), 0);
}

/// Imported value aliases retain the generic record's complete field metadata.
#[test]
fn should_omit_a_coincidental_module_for_an_imported_generic_record_alias() {
    check("module app;\nimport { record as graph } from records;\n\
        func main(): i32 { return graph.math.answer(); }\n",
        &[("records.clo", "module records;\nexport struct Pair<T> { math: T; }\n\
            export struct Leaf { answer: func(): i32; }\nfunc zero(): i32 { return 0; }\n\
            export let record: Pair<Leaf> = Pair<Leaf> { math: Leaf { answer: zero } };\n")], false);
}

/// The declaration and the complete member types can live in merged files.
#[test]
fn should_omit_a_coincidental_module_for_a_record_in_a_merged_module() {
    check("module app;\nfunc main(): i32 { return graph.math.answer(); }\n",
        &[("record.clo", "module app;\nlet graph: Pair = Pair { math: Leaf { answer: zero } };\n"),
          ("types.clo", "module app;\nstruct Leaf { answer: func(): i32; }\n\
            struct Pair { math: Leaf; }\nfunc zero(): i32 { return 0; }\n")], false);
}

/// Raw pointee metadata establishes members without dereferencing during selection.
#[test]
fn should_omit_a_coincidental_module_for_a_complete_raw_pointee_chain() {
    check("module app;\nstruct Leaf { answer: func(): i32; }\n\
        struct Pair { math: Leaf; }\nfunc zero(): i32 { return 0; }\n\
        let record: Pair = Pair { math: Leaf { answer: zero } };\n\
        let graph: raw Pair = raw Pair(0);\n\
        func main(): i32 { graph = unsafe &record; return unsafe graph.math.answer(); }\n",
        &[], false);
}

/// Lexical locals still shadow modules, unlike unrelated nonlocal declarations.
#[test]
fn should_keep_a_complete_local_chain_out_of_the_module_closure() {
    check("module app;\nstruct Leaf { answer: func(): i32; }\n\
        struct Pair { math: Leaf; }\nfunc zero(): i32 { return 0; }\n\
        func main(): i32 { let graph = Pair { math: Leaf { answer: zero } }; \
        return graph.math.answer(); }\n", &[], false);
}

/// Existing short nonlocal fields remain a compatibility control.
#[test]
fn should_keep_a_short_global_field_out_of_the_module_closure() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nstruct Pair { item: i32; }\n\
        let value: Pair = Pair { item: 0 };\nfunc main(): i32 { return value.item; }\n");
    fixture.write("coincidental.clo", "module value;\nexport const item: i32 = 42;\n");
    exit(&driver(&fixture, "run", "host", &[entry.clone()]), 0);
    let mut client = fixture.client();
    let paths = selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
    exit(&driver(&fixture, "run", "host", &paths), 0);
}

/// Existing short imported fields must not acquire a coincidental module.
#[test]
fn should_keep_a_short_import_alias_field_out_of_the_module_closure() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nimport { record as value } from records;\n\
        func main(): i32 { return value.item; }\n");
    let records = fixture.write("records.clo", "module records;\n\
        export struct Pair { item: i32; }\nexport let record: Pair = Pair { item: 0 };\n");
    fixture.write("coincidental.clo", "module value;\nexport const item: i32 = 42;\n");
    exit(&driver(&fixture, "run", "host", &[entry.clone(), records.clone()]), 0);
    let mut client = fixture.client();
    let paths = selected(&select(&mut client, &entry, &fixture.root), &entry, &[records]);
    exit(&driver(&fixture, "run", "host", &paths), 0);
}
