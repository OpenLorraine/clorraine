//! Command-line specification tests for the compile-fail golden suite.
//!
//! Every file under `spec/tests/compile-fail/` states its expectation on the
//! first line: `// expect-error CODE` or `// expect-warning CODE`. The driver
//! spawns the built `clc` binary on each file that belongs to the implemented
//! front end and checks three things:
//!
//! 1. the process exit code matches the expectation;
//! 2. a diagnostic line for the expected code exists;
//! 3. its message matches the registry template from the diagnostics chapter,
//!    where `{...}` placeholders match any text.
//!
//! Files that exercise staged or not-yet-implemented behavior are reported by
//! name and reason instead of being asserted, so the suite stays green while
//! the remaining gap stays visible.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Diagnostic codes the in-scope stages currently report.
///
/// The list grows as later stages land; every code listed here must have at
/// least one golden file in the compile-fail suite. The ownership, borrow, FFI,
/// and unsafe codes come first; the type, constant, enum, and template codes
/// follow, grouped with the front-end codes they build on: `E10213`/`E10214`
/// stay out because data inclusion is staged out, and the retired `E10225` has
/// no trigger and no golden. The warning codes come last; a warning golden
/// exits zero in the default build and nonzero once `--deny-warnings` is set.
const ASSERTED_CODES: &[&str] = &[
    "E10010", "E10011", "E10012", "E10020", "E10021", "E10022", "E10023", "E10024", "E10025",
    "E10026", "E10030", "E10031", "E10032", "E10033", "E10034", "E10035", "E10036", "E10038",
    "E10039", "E10040", "E10041", "E10042", "E10043", "E10100", "E10101", "E10102", "E10103", "E10104",
    "E10200", "E10201", "E10202", "E10203", "E10204", "E10205", "E10206", "E10207", "E10208",
    "E10209", "E10210", "E10211", "E10212", "E10215", "E10216", "E10217", "E10218", "E10219", "E10220",
    "E10221", "E10222", "E10223", "E10224", "E10226", "E10300", "E10301", "E10302", "E10303",
    "E10304", "E10305", "E10306", "E10307", "E10308", "E10400", "E10401", "E10402", "E10403",
    "E10404",
    "E10405", "E10406", "E10500",
    "E10501", "E10503", "E10600", "E10601", "E10602", "E10603", "E10604", "E11100",
    "E11101", "E11102", "E11103", "E11104", "E11105", "E11106", "E11200", "E11201", "E20000",
    "E20001", "E20002", "E20003", "E20004", "E20005", "E20006", "E20007", "E20008", "E90000",
    "E90001", "E90005", "W10000", "W10001", "W10002", "W10003", "W10004", "W10006",
];

/// Golden codes excluded from the suite because the language feature is staged
/// out of the MVP; each entry carries the reason shown in the skip report.
const EXCLUDED_CODES: &[(&str, &str)] = &[
    ("E10213", "data inclusion is not part of the implemented language subset"),
    ("E10214", "data inclusion is not part of the implemented language subset"),
];

/// One golden file plus the expectations parsed from its annotation lines.
struct Golden {
    /// Absolute path to the `.clo` source.
    path: PathBuf,
    /// File name shown in reports.
    name: String,
    /// The diagnostic code the file expects.
    code: String,
    /// True when the first line is `// expect-warning`.
    warning: bool,
    /// Extra command-line arguments from a `// cli:` annotation.
    cli: Vec<String>,
    /// Profile from a `// profile:` annotation, `host` when absent.
    profile: String,
    /// Stage name from a `// stage:` annotation, when the file waits for a
    /// language feature that is not implemented.
    stage: Option<String>,
}

/// Root of the frozen specification, resolved from the crate location so the
/// tests run from any working directory.
fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec")
}

/// Reads every `.clo` file in a directory, sorted by file name for stable
/// reports.
fn discover(dir: &Path) -> Vec<Golden> {
    let mut goldens: Vec<Golden> = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", dir.display()))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "clo"))
        .map(|path| parse_golden(&path))
        .collect();
    goldens.sort_by(|a, b| a.name.cmp(&b.name));
    goldens
}

/// Parses the expectation and optional annotations of one golden file.
///
/// Annotation lines are the leading `// ` lines after the expectation; the
/// first other line ends them. Panics when the expectation is missing: the
/// specification check enforces the file shape, so a violation here is a
/// broken fixture, not a compiler defect.
fn parse_golden(path: &Path) -> Golden {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
    let name = path
        .file_name()
        .expect("golden file name")
        .to_string_lossy()
        .into_owned();
    let first = text.lines().next().unwrap_or_default();
    let (warning, code) = if let Some(code) = first.strip_prefix("// expect-error ") {
        (false, code.trim().to_owned())
    } else if let Some(code) = first.strip_prefix("// expect-warning ") {
        (true, code.trim().to_owned())
    } else {
        panic!("{name}: first line must be `// expect-error CODE` or `// expect-warning CODE`");
    };
    let mut golden = Golden {
        path: path.to_owned(),
        name,
        code,
        warning,
        cli: Vec::new(),
        profile: "host".to_owned(),
        stage: None,
    };
    for line in text.lines().skip(1) {
        let Some(annotation) = line.strip_prefix("// ") else {
            break;
        };
        if let Some(cli) = annotation.strip_prefix("cli: ") {
            golden.cli = cli.split_whitespace().map(str::to_owned).collect();
        } else if let Some(profile) = annotation.strip_prefix("profile: ") {
            golden.profile = profile.trim().to_owned();
        } else if let Some(stage) = annotation.strip_prefix("stage: ") {
            golden.stage = Some(stage.trim().to_owned());
        }
    }
    golden
}

/// Explains why a golden is not asserted, or returns `None` when it is.
fn skip_reason(golden: &Golden) -> Option<String> {
    if let Some(stage) = &golden.stage {
        return Some(format!("activates only at its language stage ({stage})"));
    }
    if let Some((_, reason)) = EXCLUDED_CODES.iter().find(|(code, _)| *code == golden.code) {
        return Some((*reason).to_owned());
    }
    if !ASSERTED_CODES.contains(&golden.code.as_str()) {
        return Some("not covered by the implemented front end yet".to_owned());
    }
    None
}

/// Loads the message templates registered in the diagnostics chapter.
///
/// Each registry row is a markdown table row `| CODE | trigger | template |`.
/// Rows whose template cell cannot be read are left out of the map; the caller
/// then falls back to asserting the code alone.
fn load_templates(spec: &Path) -> BTreeMap<String, String> {
    let path = spec.join("14-diagnostics.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
    let mut templates = BTreeMap::new();
    for line in text.lines() {
        let cells: Vec<&str> = line.trim().split('|').map(str::trim).collect();
        if cells.len() < 4 || !is_diagnostic_code(cells[1]) {
            continue;
        }
        let template = cells[3].trim_matches('`');
        if !template.is_empty() {
            templates.insert(cells[1].to_owned(), template.to_owned());
        }
    }
    templates
}

/// True for registry codes such as `E20000` or `W10004`.
fn is_diagnostic_code(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some('E' | 'W'))
        && text.len() == 6
        && chars.all(|c| c.is_ascii_digit())
}

/// Matches a rendered diagnostic message against a registry template.
///
/// A `{...}` placeholder matches any text, preferring the shortest match that
/// still lets the remaining literal text line up; every other character is
/// literal. Trailing placeholders match the rest of the message.
fn message_matches(template: &str, message: &str) -> bool {
    let mut rest_template = template;
    let mut rest_message = message;
    while let Some(open) = rest_template.find('{') {
        let literal = &rest_template[..open];
        let Some(message_tail) = rest_message.strip_prefix(literal) else {
            return false;
        };
        rest_message = message_tail;
        let Some(close) = rest_template[open..].find('}') else {
            return false;
        };
        rest_template = &rest_template[open + close + 1..];
        if rest_template.is_empty() {
            return true;
        }
        let next_open = rest_template.find('{').unwrap_or(rest_template.len());
        let next_literal = &rest_template[..next_open];
        if next_literal.is_empty() {
            continue;
        }
        let Some(offset) = rest_message.find(next_literal) else {
            return false;
        };
        rest_message = &rest_message[offset..];
    }
    rest_template == rest_message
}

/// True when the line starts a rendered diagnostic (`error CODE: ...` or
/// `warning CODE: ...`).
fn is_diagnostic_head(line: &str) -> bool {
    line.starts_with("error ") || line.starts_with("warning ")
}

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Creates a unique directory for one test process and tag.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-spec-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    /// Path of the scratch directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Checks one golden file and returns the failure reason, if any.
fn check_golden(
    golden: &Golden,
    templates: &BTreeMap<String, String>,
    scratch: &Path,
) -> Result<(), String> {
    let output_file = scratch.join(format!("{}.c", golden.name));
    let mut args = vec!["--profile".to_owned(), golden.profile.clone()];
    args.extend(golden.cli.iter().cloned());
    args.push("-o".to_owned());
    args.push(output_file.display().to_string());
    args.push(golden.path.display().to_string());
    let run = run_clc(&args);
    let stderr = String::from_utf8_lossy(&run.stderr);
    for line in stderr.lines().filter(|line| !line.is_empty()) {
        if !is_diagnostic_head(line) && !line.starts_with("  ") {
            return Err(format!("unexpected compiler output line {line:?}"));
        }
    }
    let head = format!(
        "{} {}: ",
        if golden.warning { "warning" } else { "error" },
        golden.code
    );
    let Some(line) = stderr.lines().find(|line| line.starts_with(&head)) else {
        return Err(format!(
            "no `{head}...` diagnostic line (exit {:?})",
            run.status.code()
        ));
    };
    if let Some(template) = templates.get(&golden.code) {
        let message = &line[head.len()..];
        if !message_matches(template, message) {
            return Err(format!(
                "message {message:?} does not match template {template:?}"
            ));
        }
    }
    let deny = golden.cli.iter().any(|arg| arg == "--deny-warnings");
    let expected_exit = if golden.warning && !deny { 0 } else { 1 };
    let actual_exit = run.status.code().unwrap_or(-1);
    if actual_exit != expected_exit {
        return Err(format!("exit code {actual_exit}, expected {expected_exit}"));
    }
    Ok(())
}

/// Runs every asserted golden and reports all failures at once.
#[test]
fn compile_fail_goldens() {
    let spec = spec_root();
    let templates = load_templates(&spec);
    let goldens = discover(&spec.join("tests/compile-fail"));
    let scratch = ScratchDir::new("cf");
    let mut asserted = 0;
    let mut failures = Vec::new();
    for golden in &goldens {
        if !ASSERTED_CODES.contains(&golden.code.as_str()) {
            continue;
        }
        asserted += 1;
        if let Err(reason) = check_golden(golden, &templates, scratch.path()) {
            failures.push(format!("{}: {reason}", golden.name));
        }
    }
    assert!(
        asserted >= ASSERTED_CODES.len(),
        "only {asserted} asserted goldens found for {} codes; a golden file is missing",
        ASSERTED_CODES.len()
    );
    assert!(
        failures.is_empty(),
        "{} of {asserted} asserted goldens failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Every warning golden escalates to a failed build under `--deny-warnings`,
/// and its message stays a `warning CODE:` line rather than becoming an error.
///
/// The default-mode warning goldens already prove the warning is reported and
/// the build succeeds; this case proves the same warnings are fatal under the
/// escalation flag without changing their rendered severity.
#[test]
fn every_warning_escalates_under_deny_warnings() {
    let spec = spec_root();
    let goldens = discover(&spec.join("tests/compile-fail"));
    let scratch = ScratchDir::new("deny");
    let mut checked = 0;
    let mut failures = Vec::new();
    for golden in goldens.iter().filter(|golden| golden.warning) {
        if golden.cli.iter().any(|arg| arg == "--deny-warnings") {
            // The dedicated deny golden already asserts this behavior.
            continue;
        }
        let output_file = scratch.path().join(format!("deny-{}.c", golden.name));
        let run = run_clc(&[
            "--profile".to_owned(),
            golden.profile.clone(),
            "--deny-warnings".to_owned(),
            "-o".to_owned(),
            output_file.display().to_string(),
            golden.path.display().to_string(),
        ]);
        if run.status.code() != Some(1) {
            failures.push(format!(
                "{}: exit {:?}, expected 1",
                golden.name,
                run.status.code()
            ));
            continue;
        }
        let stderr = String::from_utf8_lossy(&run.stderr);
        let head = format!("warning {}: ", golden.code);
        let first = stderr.lines().next().unwrap_or_default();
        if !first.starts_with(&head) {
            failures.push(format!(
                "{}: first line {first:?} is not `{head}...`",
                golden.name
            ));
            continue;
        }
        checked += 1;
    }
    assert!(checked >= 5, "only {checked} warning goldens were checked");
    assert!(
        failures.is_empty(),
        "{} warning goldens did not escalate:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Prints the skip report: which goldens are asserted and which are skipped
/// with what reason, plus the aggregate counts.
#[test]
fn skip_report() {
    let spec = spec_root();
    let mut asserted = 0usize;
    let mut excluded: Vec<(String, String)> = Vec::new();
    let mut pending: Vec<(String, String)> = Vec::new();
    for golden in discover(&spec.join("tests/compile-fail")) {
        let known_exclusion = golden.stage.is_some()
            || EXCLUDED_CODES.iter().any(|(code, _)| *code == golden.code);
        match skip_reason(&golden) {
            None => asserted += 1,
            Some(reason) if known_exclusion => excluded.push((golden.name.clone(), reason)),
            Some(reason) => pending.push((golden.name.clone(), reason)),
        }
    }
    let staged_dir = spec.join("tests/staged");
    let mut staged_suite: Vec<String> = std::fs::read_dir(&staged_dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", staged_dir.display()))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "clo"))
        .map(|path| path.file_name().expect("file name").to_string_lossy().into_owned())
        .collect();
    staged_suite.sort();
    for name in &staged_suite {
        excluded.push((
            name.clone(),
            "staged suite: waits for its language feature".to_owned(),
        ));
    }
    println!("compile-fail skip report:");
    println!("  asserted: {asserted}");
    println!("  excluded: {}", excluded.len());
    for (name, reason) in &excluded {
        println!("    {name}: {reason}");
    }
    println!("  pending: {}", pending.len());
    for (name, reason) in &pending {
        println!("    {name}: {reason}");
    }
    println!("  skipped: {}", excluded.len() + pending.len());
    for (name, reason) in excluded.iter().chain(pending.iter()) {
        assert!(!reason.is_empty(), "{name}: a skip reason is required");
    }
    assert!(
        pending.is_empty(),
        "the accepted front end must cover every in-scope golden, but {} remain pending: {pending:?}",
        pending.len()
    );
    assert!(
        !excluded.is_empty(),
        "the staged and excluded files must appear in the skip report"
    );
}
