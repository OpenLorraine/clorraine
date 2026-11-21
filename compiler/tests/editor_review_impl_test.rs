//! Declared shape boundaries used by saved-source member classification.
//! The compiler still judges expressions; these real-process controls ensure
//! view fields and recursive types do not need a second inference engine.

#[allow(dead_code)]
mod editor_sources_support;
#[allow(dead_code)]
mod lsp_support;

use editor_sources_support::{driver, exit, select, selected, Fixture};

/// A complete declared chain must keep the coincidental module out of argv.
fn field_chain(source: &str, coincidental: &str) {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", source);
    fixture.write("coincidental.clo", coincidental);
    exit(&driver(&fixture, "run", "host", &[entry.clone()]), 0);
    let mut client = fixture.client();
    let paths = selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
    exit(&driver(&fixture, "run", "host", &paths), 0);
}

/// Constant view shapes reuse the same member table as ordinary globals.
#[test]
fn should_keep_a_static_view_length_out_of_the_module_closure() {
    field_chain(
        "module app;\nconst graph: static str = \"ok\";\n\
         func main(): i32 { return i32(graph.len) - 2; }\n",
        "module graph;\nexport const len: i32 = 42;\n",
    );
}

/// Fixed arrays have a length member without needing their extent evaluated here.
#[test]
fn should_follow_a_declared_array_field_to_its_length_member() {
    field_chain(
        "module app;\nconst N: usize = 2;\nstruct Row { items: i32[N]; }\n\
         let graph: Row = Row { items: [0, 0] };\n\
         func main(): i32 { return i32(graph.items.len) - 2; }\n",
        "module graph.items;\nexport const len: i32 = 42;\n",
    );
}

/// Recursive structs advance only along the finite written path being queried.
#[test]
fn should_follow_a_finite_chain_through_a_recursive_raw_field() {
    field_chain(
        "module app;\nstruct Node { next: raw Node; answer: i32; }\n\
         let graph: Node = Node { next: raw Node(0), answer: 0 };\n\
         func main(): i32 { graph.next = unsafe &graph; \
         return unsafe graph.next.next.answer; }\n",
        "module graph.next.next;\nexport const answer: i32 = 42;\n",
    );
}
