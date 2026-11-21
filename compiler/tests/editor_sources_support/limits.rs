//! Public file-count boundaries, separate from protocol-message byte limits.

use std::path::PathBuf;

use super::{rejected, select, selected, Fixture};

/// Writes a known chain of distinct modules; the last file has a literal tail.
///
/// The returned paths are the expected fixture list, not a discovered closure.
fn chain(fixture: &Fixture, count: usize, tail: &str) -> Vec<PathBuf> {
    (0..count)
        .map(|index| {
            let body = if index + 1 < count {
                format!(
                    "import {{ pick as next }} from chain.m{};\n\
                     export func pick(): i32 {{ return next(); }}\n",
                    index + 1
                )
            } else {
                tail.to_owned()
            };
            let main = if index == 0 {
                "func main(): i32 { return pick(); }\n"
            } else {
                ""
            };
            fixture.write(
                &format!("chain-{index:04}.clo"),
                &format!("module chain.m{index};\n{body}{main}"),
            )
        })
        .collect()
}

/// An index can be full even when the selected program consists of one file.
#[test]
fn should_accept_a_complete_index_of_exactly_2000_files() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nfunc main(): i32 { return 0; }\n");
    for index in 0..1999 {
        fixture.write(
            &format!("inventory/{index:04}.clo"),
            &format!("module spare.m{index};\nexport const item: i32 = 0;\n"),
        );
    }
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
}

/// Overflow cannot be mistaken for successfully choosing a small entry program.
#[test]
fn should_reject_the_2001st_indexable_source_without_a_partial_list() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nfunc main(): i32 { return 0; }\n");
    for index in 0..2000 {
        fixture.write(
            &format!("inventory/{index:04}.clo"),
            &format!("module spare.m{index};\nexport const item: i32 = 0;\n"),
        );
    }
    let mut client = fixture.client();
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// A complete 256-file program fits even though its wire response has many bytes.
#[test]
fn should_accept_exactly_256_selected_files_in_entry_first_order() {
    let fixture = Fixture::new();
    let paths = chain(&fixture, 256, "export func pick(): i32 { return 0; }\n");
    let mut client = fixture.client();
    selected(
        &select(&mut client, &paths[0], &fixture.root),
        &paths[0],
        &paths[1..],
    );
}

/// The index is far below 2000, so only the selected-file budget can reject this.
#[test]
fn should_reject_a_257th_file_discovered_from_the_256th_selected_file() {
    let fixture = Fixture::new();
    let paths = chain(&fixture, 257, "export func pick(): i32 { return 0; }\n");
    let mut client = fixture.client();
    rejected(&select(&mut client, &paths[0], &fixture.root));
}

/// Reaching 256 paths is not proof that the final path's imports were processed.
#[test]
fn should_check_a_missing_import_in_the_256th_selected_file() {
    let fixture = Fixture::new();
    let paths = chain(
        &fixture,
        256,
        "import { absent } from missing.final;\nexport func pick(): i32 { return absent(); }\n",
    );
    let mut client = fixture.client();
    rejected(&select(&mut client, &paths[0], &fixture.root));
}

/// Same-module merging is also subject to the selected-file bound.
#[test]
fn should_reject_257_files_merged_into_the_entry_module() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nfunc main(): i32 { return 0; }\n");
    for index in 0..256 {
        fixture.write(
            &format!("part-{index:04}.clo"),
            &format!("module app;\nconst item{index}: i32 = 0;\n"),
        );
    }
    let mut client = fixture.client();
    rejected(&select(&mut client, &entry, &fixture.root));
}

/// Generated, hidden and symlinked directory contents do not consume the index.
#[test]
fn should_exclude_skipped_directories_from_the_index_budget() {
    let fixture = Fixture::new();
    let entry = fixture.write("entry.clo", "module app;\nfunc main(): i32 { return 0; }\n");
    for directory in [".git", "build", "target", "node_modules", ".hidden"] {
        for index in 0..2001 {
            fixture.write(
                &format!("{directory}/{index:04}.clo"),
                &format!("module ignored.m{index};\nconst item: i32 = 0;\n"),
            );
        }
    }
    let external = Fixture::new();
    for index in 0..2001 {
        external.write(
            &format!("{index:04}.clo"),
            &format!("module external.m{index};\nconst item: i32 = 0;\n"),
        );
    }
    std::os::unix::fs::symlink(&external.root, fixture.root.join("linked"))
        .expect("symlinked directory fixture");
    let mut client = fixture.client();
    selected(&select(&mut client, &entry, &fixture.root), &entry, &[]);
}
