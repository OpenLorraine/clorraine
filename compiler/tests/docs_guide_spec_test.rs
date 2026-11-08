//! Specification tests for the C-to-Clorraine guide and documentation links.
//!
//! The guide is a documentation deliverable, not code. These cases pin the
//! required sections, the diagnostic table, and every relative link so the
//! guide cannot silently lose a pattern or point at a missing file.

use std::path::{Path, PathBuf};

/// The repository root from the compiler crate.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root")
        .to_path_buf()
}

/// Reads a repository file.
fn read(relative: &str) -> String {
    std::fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"))
}

#[test]
fn should_cover_every_required_pattern() {
    let guide = read("docs/c-to-clorraine.md");
    for heading in [
        "## 1. Mental model",
        "## 2. Allocation",
        "## 3. Files",
        "## 4. Strings",
        "## 5. MMIO",
        "## 6. Interrupts",
        "## 7. Hooks",
        "## 8. Ownership modes",
        "## 9. Error handling",
        "## 10. Diagnostics a C programmer will meet",
        "## 11. Wrapper cookbook",
    ] {
        assert!(guide.contains(heading), "missing heading: {heading}");
    }
}

#[test]
fn should_name_every_required_diagnostic() {
    let guide = read("docs/c-to-clorraine.md");
    for code in [
        "E10010", "E10012", "E10020", "E10026", "E10034", "E10105", "E10201",
        "E10207", "E10306", "E10400", "E11200", "E11215",
    ] {
        assert!(guide.contains(code), "missing diagnostic: {code}");
    }
}

#[test]
fn should_link_the_guide_from_the_docs_index_and_readme() {
    assert!(read("docs/README.md").contains("c-to-clorraine.md"));
    assert!(read("README.md").contains("docs/c-to-clorraine.md"));
}

#[test]
fn should_resolve_every_relative_link() {
    for relative in ["docs/c-to-clorraine.md", "docs/README.md", "README.md"] {
        let path = repo_root().join(relative);
        let text = std::fs::read_to_string(&path).expect("document");
        let directory = path.parent().expect("parent");
        let mut rest = text.as_str();
        while let Some(open) = rest.find("](") {
            rest = &rest[open + 2..];
            let Some(close) = rest.find(')') else {
                break;
            };
            let target = &rest[..close];
            rest = &rest[close + 1..];
            if target.starts_with("http")
                || target.starts_with("mailto:")
                || target.starts_with('#')
                || target.is_empty()
            {
                continue;
            }
            let target = target.split('#').next().unwrap_or("");
            if target.is_empty() {
                continue;
            }
            assert!(
                directory.join(target).exists(),
                "{relative}: broken link {target}"
            );
        }
    }
}
