//! Implementation tests for the C spelling of source names.
//!
//! The emitter renames a source name that would break or change the emitted
//! C: a C keyword, a reserved identifier, or a name the translation unit's
//! headers and runtime define or declare. The last class is data, the sorted
//! list in `compiler/src/codegen/c_names.txt`, because emission must not depend
//! on the machine that builds it.
//!
//! The completeness cases re-derive that list. Each compile-matrix leg emits a
//! fixed sample program, preprocesses the whole unit with the leg's own
//! compiler and flags, and collects every ordinary identifier the unit
//! defines or declares at file scope: macros from `-dM`, functions from
//! `-aux-info`, and typedefs, objects, functions, and enumerators from a scan
//! of the preprocessed text. A name the list lacks fails the case, which
//! prints each missing name on its own `missing:` line so the list can be
//! extended from the log. A leg whose compiler is absent skips itself.
//!
//! The rc24 product toolchain needs a newer C library than the SDK image, so
//! its case runs only when `CLORRAINE_RC24_ROOT` names a toolchain root; the
//! case builds its sample through the compiler library, so a binary built in
//! the image can run it on the host.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The committed list of header and runtime names, one per line, sorted.
const C_NAMES: &str = include_str!("../src/codegen/c_names.txt");

/// The fixed program every leg emits.
///
/// It imports nothing, so the unit holds exactly the runtime prefix, the
/// emitter's fixed typedefs and helpers, and, on hosted-amiga, the runtime
/// suffix; nothing in it depends on the standard library's contents.
const SAMPLE: &str = "module Sample;\n\nfunc main(): i32 {\n    return 0;\n}\n";

/// Spellings that are the sample program's own symbols, not header names.
///
/// The emitter mangles the program's functions, objects, and initializers
/// with these prefixes; a source name can never take one of them by accident
/// of a header, so they stay out of the list.
const PROGRAM_PREFIXES: [&str; 3] = ["cl_Sample_", "cl_init_", "cl_std_"];

/// The C99 keywords, plus `bool`, which every leg's `<stdbool.h>` defines.
///
/// A keyword is escaped on its own, so a leg that reports one (a `bool`
/// macro) does not need a list entry for it.
const C_KEYWORDS: [&str; 38] = [
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
    "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
    "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef",
    "union", "unsigned", "void", "volatile", "while", "_Bool", "_Complex", "_Imaginary", "bool",
];

/// The vbcc target headers inside the SDK image.
const VBCC_INCLUDE: &str = "/opt/amiga/m68k-amigaos/vbcc/include";

/// The NDK headers inside the SDK image.
const NDK_INCLUDE: &str = "/opt/amiga/m68k-amigaos/ndk-include";

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
            "clc-c-names-impl-{}-{tag}-{sequence}",
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

/// True when the C compiler `tool` starts and answers `--version`.
///
/// Only compilers are probed this way: `vamos` rejects `--version`, so it is
/// found on `PATH` with [`on_path`] instead.
fn tool_available(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// True when `tool` names a file in one of the `PATH` directories.
fn on_path(tool: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(tool).is_file()))
}

/// How one compile-matrix leg reads the emitted unit.
struct Leg {
    /// Short name printed beside each missing name.
    name: &'static str,
    /// The `clc` profile the sample is emitted for.
    profile: &'static str,
    /// The C compiler executable.
    compiler: String,
    /// The leg's fixed flags: the compile matrix's base line, or for vbcc the
    /// preprocessor options that reproduce its known predefines and include
    /// order.
    flags: Vec<String>,
    /// True when the compiler can also report function prototypes with
    /// `-aux-info`; the vbcc leg only borrows GCC's preprocessor.
    aux_info: bool,
    /// A header directory whose every `#define` name also counts, for a
    /// compiler whose own predefines cannot be listed.
    define_dir: Option<&'static str>,
    /// The optimization flags the unit is read with, one extraction each.
    ///
    /// The debug, release, and size lines define different macros
    /// (`__OPTIMIZE__`, `__NO_INLINE__`), and headers select different code
    /// on them, so a GCC leg is read at every level the compile matrix uses.
    levels: &'static [&'static str],
}

/// The optimization levels of the compile matrix's debug, release, and size
/// lines.
const GCC_LEVELS: &[&str] = &["-O0", "-O2", "-Os"];

/// Turns a slice of flag literals into owned arguments.
fn owned(flags: &[&str]) -> Vec<String> {
    flags.iter().map(|flag| (*flag).to_owned()).collect()
}

/// The legs the SDK image provides, in the order they are reported.
fn sdk_legs() -> Vec<Leg> {
    let m68k = "m68k-amigaos-gcc".to_owned();
    vec![
        Leg {
            name: "host",
            profile: "host",
            compiler: "cc".to_owned(),
            flags: owned(&["-std=c99"]),
            aux_info: true,
            define_dir: None,
            levels: GCC_LEVELS,
        },
        Leg {
            name: "host-windows",
            profile: "host",
            compiler: "x86_64-w64-mingw32-gcc".to_owned(),
            flags: owned(&["-std=c99"]),
            aux_info: true,
            define_dir: None,
            levels: GCC_LEVELS,
        },
        Leg {
            name: "hosted-amiga",
            profile: "hosted-amiga",
            compiler: m68k.clone(),
            flags: owned(&["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv"]),
            aux_info: true,
            define_dir: None,
            levels: GCC_LEVELS,
        },
        Leg {
            name: "hosted-amiga-13",
            profile: "hosted-amiga-13",
            compiler: m68k.clone(),
            flags: owned(&["-std=c99", "-mcrt=nix13", "-m68000", "-fwrapv"]),
            aux_info: true,
            define_dir: None,
            levels: GCC_LEVELS,
        },
        Leg {
            name: "freestanding",
            profile: "freestanding",
            compiler: m68k.clone(),
            flags: owned(&[
                "-std=c99",
                "-ffreestanding",
                "-fno-builtin",
                "-nostdlib",
                "-m68000",
                "-fwrapv",
            ]),
            aux_info: true,
            define_dir: None,
            levels: GCC_LEVELS,
        },
        Leg {
            name: "freestanding-host",
            profile: "freestanding",
            compiler: "cc".to_owned(),
            flags: owned(&["-std=c99", "-ffreestanding", "-fno-builtin", "-nostdlib"]),
            aux_info: true,
            define_dir: None,
            levels: GCC_LEVELS,
        },
        // vbcc has no `-dM`, so GCC's preprocessor reads the unit the way the
        // compile matrix's `vc +aos68k` line does: the `aos68k` configuration
        // searches the command line's NDK directory and then vbcc's own target
        // headers, and `vbccm68k -cpu=68000` predefines these macros. Its other
        // predefines are reserved-shaped and escaped by their shape.
        Leg {
            name: "vbcc",
            profile: "hosted-amiga",
            compiler: m68k,
            flags: vec![
                "-std=c99".to_owned(),
                "-nostdinc".to_owned(),
                "-undef".to_owned(),
                "-D__VBCC__".to_owned(),
                "-D__M68K__".to_owned(),
                "-D__M68000=1".to_owned(),
                "-D__INTSIZE=32".to_owned(),
                format!("-I{NDK_INCLUDE}"),
                format!("-I{VBCC_INCLUDE}"),
            ],
            aux_info: false,
            define_dir: Some(VBCC_INCLUDE),
            levels: &[],
        },
    ]
}

/// The two hosted legs of the rc24 toolchain rooted at `root`.
fn rc24_legs(root: &Path) -> Vec<Leg> {
    let compiler = root.join("bin/m68k-amigaos-gcc").display().to_string();
    vec![
        Leg {
            name: "rc24 hosted-amiga",
            profile: "hosted-amiga",
            compiler: compiler.clone(),
            flags: owned(&["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv"]),
            aux_info: true,
            define_dir: None,
            levels: GCC_LEVELS,
        },
        Leg {
            name: "rc24 hosted-amiga-13",
            profile: "hosted-amiga-13",
            compiler,
            flags: owned(&["-std=c99", "-mcrt=nix13", "-m68000", "-fwrapv"]),
            aux_info: true,
            define_dir: None,
            levels: GCC_LEVELS,
        },
    ]
}

/// Emits the sample for `profile` through the compiler library.
///
/// The library call needs no compiler binary or repository path, so the same
/// test binary works inside the SDK image and on a host.
fn emit_sample(scratch: &ScratchDir, profile: &str) -> PathBuf {
    let source = scratch.path().join("sample.clo");
    std::fs::write(&source, SAMPLE).expect("write the sample program");
    let c_path = scratch.path().join(format!("sample-{profile}.c"));
    let status = clc::driver::run(vec![
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source.display().to_string(),
    ]);
    assert_eq!(status, 0, "the sample must compile for {profile}");
    c_path
}

/// Runs a leg's compiler with extra arguments and returns its stdout.
fn run_compiler(leg: &Leg, extra: &[&str], c_path: &Path) -> String {
    let output = Command::new(&leg.compiler)
        .args(&leg.flags)
        .args(extra)
        .arg(c_path)
        .output()
        .unwrap_or_else(|err| panic!("{}: cannot run {}: {err}", leg.name, leg.compiler));
    assert!(
        output.status.success(),
        "{}: {} {:?} failed; stderr:\n{}",
        leg.name,
        leg.compiler,
        extra,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The identifier a `#define` line defines, if the line is one.
///
/// Leading blanks and blanks after `#` are allowed, as in a header; the name
/// ends at the first byte that cannot continue an identifier, so a
/// function-like macro yields its name without the parameter list.
fn defined_name(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix('#')?.trim_start();
    let rest = rest.strip_prefix("define")?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim_start();
    let end = rest
        .find(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .unwrap_or(rest.len());
    let name = &rest[..end];
    (!name.is_empty() && !name.starts_with(|ch: char| ch.is_ascii_digit())).then_some(name)
}

/// Every `#define` name in every file below `dir`.
fn define_names_under(dir: &Path, names: &mut BTreeSet<String>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", dir.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            define_names_under(&path, names);
        } else if let Ok(bytes) = std::fs::read(&path) {
            for line in String::from_utf8_lossy(&bytes).lines() {
                if let Some(name) = defined_name(line) {
                    names.insert(name.to_owned());
                }
            }
        }
    }
}

/// One token of preprocessed C, as far as the declaration scan needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    /// An identifier or keyword.
    Ident(String),
    /// One punctuation byte.
    Punct(char),
    /// A number, string, or character literal; its text never matters.
    Literal,
}

/// Splits preprocessed C into tokens.
///
/// Line markers and pragmas (lines whose first non-blank byte is `#`) are
/// dropped. String and character literals become one opaque token each, so a
/// brace or semicolon inside one never moves the scan.
fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let bytes = line.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            if byte.is_ascii_whitespace() {
                index += 1;
            } else if byte == b'_' || byte == b'$' || byte.is_ascii_alphabetic() {
                let start = index;
                while index < bytes.len()
                    && (bytes[index] == b'_' || bytes[index] == b'$' || bytes[index].is_ascii_alphanumeric())
                {
                    index += 1;
                }
                tokens.push(Token::Ident(line[start..index].to_owned()));
            } else if byte.is_ascii_digit()
                || (byte == b'.' && bytes.get(index + 1).is_some_and(u8::is_ascii_digit))
            {
                index += 1;
                while index < bytes.len() {
                    let next = bytes[index];
                    let exponent_sign = (next == b'+' || next == b'-')
                        && matches!(bytes[index - 1], b'e' | b'E' | b'p' | b'P');
                    if next == b'_' || next == b'.' || next.is_ascii_alphanumeric() || exponent_sign {
                        index += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Literal);
            } else if byte == b'"' || byte == b'\'' {
                index += 1;
                while index < bytes.len() && bytes[index] != byte {
                    if bytes[index] == b'\\' {
                        index += 1;
                    }
                    index += 1;
                }
                index += 1;
                tokens.push(Token::Literal);
            } else {
                tokens.push(Token::Punct(byte as char));
                index += 1;
            }
        }
    }
    tokens
}

/// Identifiers that introduce a parenthesized group the scan must skip whole:
/// attributes, inline assembly, and compile-time operators whose operands
/// name types or values but declare nothing.
const SKIP_GROUP: [&str; 9] = [
    "__attribute__",
    "__attribute",
    "__asm__",
    "__asm",
    "asm",
    "__declspec",
    "__typeof__",
    "__typeof",
    "_Static_assert",
];

/// Identifiers that qualify a declaration without being part of its name.
const QUALIFIERS: [&str; 6] = ["__extension__", "__inline__", "__inline", "__volatile__", "__restrict", "__restrict__"];

/// The index just past the group that opens at `open`, a `(`, `[`, or `{`.
fn skip_group(tokens: &[Token], open: usize) -> usize {
    let mut depth = 0usize;
    let mut index = open;
    while index < tokens.len() {
        match tokens[index] {
            Token::Punct('(' | '[' | '{') => depth += 1,
            Token::Punct(')' | ']' | '}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    tokens.len()
}

/// The index just past an attribute or assembly group that starts at `index`,
/// or `index` when no such group starts there.
fn skip_extension(tokens: &[Token], mut index: usize) -> usize {
    let Some(Token::Ident(name)) = tokens.get(index) else {
        return index;
    };
    if !SKIP_GROUP.contains(&name.as_str()) {
        return index;
    }
    index += 1;
    // `__asm volatile ( ... )` and `asm goto ( ... )` carry a qualifier
    // before the group.
    while let Some(Token::Ident(word)) = tokens.get(index) {
        if matches!(word.as_str(), "volatile" | "__volatile__" | "goto" | "inline") {
            index += 1;
        } else {
            break;
        }
    }
    if tokens.get(index) == Some(&Token::Punct('(')) {
        skip_group(tokens, index)
    } else {
        index
    }
}

/// Collects every ordinary identifier preprocessed C declares at file scope.
///
/// The scan walks declarations at brace depth zero. A declarator's name is the
/// identifier written just before `;`, `,`, `=`, `[`, `:`, or a parameter
/// list's `(`, at parenthesis depth zero or inside a `(*name)` declarator
/// group; names inside a parameter list are never taken. A tag after
/// `struct`, `union`, or `enum` lives in its own C namespace and is skipped.
/// Aggregate bodies are entered only to collect the enumerators of any enum
/// they hold; function bodies and initializers are skipped whole. The scan
/// may take a name the C compiler would not, which costs only an escape; it
/// must not miss one, which the `-aux-info` recall case checks.
fn scan_declarations(tokens: &[Token], names: &mut BTreeSet<String>) {
    let mut index = 0;
    scan_scope(tokens, &mut index, tokens.len(), names);
}

/// What kind of parenthesis group a declaration has open.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    /// A `(*name)` group that holds the declarator name.
    Declarator,
    /// A parameter list or any other group whose names are not declared.
    Other,
}

/// Scans declarations from `*index` up to `end`, the index of the closing
/// brace of an aggregate body or the end of the unit.
///
/// Inside an aggregate body the member names are not collected; only nested
/// enum bodies contribute their enumerators.
fn scan_scope(tokens: &[Token], index: &mut usize, end: usize, names: &mut BTreeSet<String>) {
    let file_scope = end == tokens.len();
    let mut groups: Vec<Group> = Vec::new();
    // The identifier most recently seen at a position that can hold a
    // declarator name, cleared by anything that cannot follow one.
    let mut candidate: Option<String> = None;
    // The previous significant token, to tell a function body from an
    // aggregate body.
    let mut previous: Option<Token> = None;
    // The keyword of a `struct`, `union`, or `enum` whose body may follow.
    let mut aggregate: Option<String> = None;
    let take = |name: Option<String>, names: &mut BTreeSet<String>| {
        if file_scope {
            if let Some(name) = name {
                names.insert(name);
            }
        }
    };
    while *index < end {
        let skipped = skip_extension(tokens, *index);
        if skipped != *index {
            *index = skipped;
            continue;
        }
        let token = tokens[*index].clone();
        let in_name_position = groups.iter().all(|group| *group == Group::Declarator);
        match &token {
            Token::Ident(word) if QUALIFIERS.contains(&word.as_str()) => {
                *index += 1;
                continue;
            }
            Token::Ident(word) if matches!(word.as_str(), "struct" | "union" | "enum") => {
                aggregate = Some(word.clone());
                candidate = None;
                *index += 1;
                // The tag, when present, names the aggregate in the tag
                // namespace, not an ordinary identifier.
                if let Some(Token::Ident(_)) = tokens.get(*index) {
                    *index += 1;
                }
                previous = Some(token);
                continue;
            }
            Token::Ident(word) => {
                candidate = in_name_position.then(|| word.clone());
            }
            Token::Punct('(') => {
                let starred = tokens.get(*index + 1) == Some(&Token::Punct('*'));
                if in_name_position && !starred {
                    // A parameter list: the identifier before it names a
                    // function (or a function type).
                    take(candidate.take(), names);
                }
                groups.push(if in_name_position && starred {
                    Group::Declarator
                } else {
                    Group::Other
                });
                candidate = None;
            }
            Token::Punct(')') => {
                if groups.last() == Some(&Group::Declarator) {
                    take(candidate.take(), names);
                }
                groups.pop();
                candidate = None;
            }
            Token::Punct('[') => {
                if in_name_position {
                    take(candidate.take(), names);
                }
                *index = skip_group(tokens, *index);
                previous = Some(Token::Punct(']'));
                continue;
            }
            Token::Punct('{') => {
                if let Some(kind) = aggregate.take() {
                    let close = skip_group(tokens, *index) - 1;
                    if kind == "enum" {
                        collect_enumerators(tokens, *index + 1, close, names);
                    } else {
                        let mut inner = *index + 1;
                        scan_scope(tokens, &mut inner, close, names);
                    }
                    *index = close + 1;
                    previous = Some(Token::Punct('}'));
                    candidate = None;
                    continue;
                }
                // A function body or a braced initializer declares nothing at
                // file scope; a function body also ends its declaration.
                let body = previous == Some(Token::Punct(')'));
                *index = skip_group(tokens, *index);
                if body && groups.is_empty() {
                    previous = None;
                    candidate = None;
                    aggregate = None;
                } else {
                    previous = Some(Token::Punct('}'));
                }
                continue;
            }
            Token::Punct(';' | ',' | ':') => {
                if in_name_position {
                    take(candidate.take(), names);
                }
                candidate = None;
                if token == Token::Punct(';') {
                    aggregate = None;
                }
            }
            Token::Punct('=') => {
                if in_name_position {
                    take(candidate.take(), names);
                }
                // Skip the initializer to the `,` or `;` that ends it.
                *index += 1;
                while *index < end {
                    match tokens[*index] {
                        Token::Punct('(' | '[' | '{') => *index = skip_group(tokens, *index),
                        Token::Punct(',' | ';') => break,
                        _ => *index += 1,
                    }
                }
                candidate = None;
                previous = Some(Token::Punct('='));
                continue;
            }
            _ => {
                candidate = None;
            }
        }
        previous = Some(token);
        *index += 1;
    }
}

/// Collects the enumerators of one enum body between `start` and `end`.
///
/// An enumerator is the first identifier of each comma-separated entry; its
/// value expression is skipped, so names it mentions are not taken.
fn collect_enumerators(tokens: &[Token], start: usize, end: usize, names: &mut BTreeSet<String>) {
    let mut index = start;
    let mut expecting = true;
    while index < end {
        let skipped = skip_extension(tokens, index);
        if skipped != index {
            index = skipped;
            continue;
        }
        match &tokens[index] {
            Token::Ident(name) if expecting => {
                names.insert(name.clone());
                expecting = false;
                index += 1;
            }
            Token::Punct('(' | '[' | '{') => index = skip_group(tokens, index),
            Token::Punct(',') => {
                expecting = true;
                index += 1;
            }
            _ => index += 1,
        }
    }
}

/// The function names an `-aux-info` report declares or defines.
///
/// Each line is one prototype after a `/* file:line:flags */` comment. The
/// name is the identifier just before the first `(` that opens a parameter
/// list rather than a `(*` declarator group.
fn aux_info_functions(report: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in report.lines() {
        let declaration = match line.find("*/") {
            Some(end) => &line[end + 2..],
            None => line,
        };
        let tokens = tokenize(declaration);
        for (index, token) in tokens.iter().enumerate() {
            if *token != Token::Punct('(') || tokens.get(index + 1) == Some(&Token::Punct('*')) {
                continue;
            }
            if let Some(Token::Ident(name)) = index.checked_sub(1).map(|before| &tokens[before]) {
                if !SKIP_GROUP.contains(&name.as_str()) {
                    names.insert(name.clone());
                    break;
                }
            }
        }
    }
    names
}

/// The names one leg's view of the unit defines or declares, split by source
/// so the recall case can compare the scan with `-aux-info`.
struct Extraction {
    /// Macro names from `-dM` and any header directory.
    macros: BTreeSet<String>,
    /// Function names from `-aux-info`; empty for a leg without it.
    functions: BTreeSet<String>,
    /// Names the declaration scan found in the preprocessed unit.
    scanned: BTreeSet<String>,
}

impl Extraction {
    /// Every name of the three sources together.
    fn all(&self) -> BTreeSet<String> {
        let mut all = self.macros.clone();
        all.extend(self.functions.iter().cloned());
        all.extend(self.scanned.iter().cloned());
        all
    }
}

/// Emits the sample for a leg and extracts every name its unit holds at every
/// optimization level the leg is read with.
fn extract(leg: &Leg) -> Extraction {
    let scratch = ScratchDir::new(&leg.name.replace(' ', "-"));
    let c_path = emit_sample(&scratch, leg.profile);
    let mut all = Extraction {
        macros: BTreeSet::new(),
        functions: BTreeSet::new(),
        scanned: BTreeSet::new(),
    };
    let levels: Vec<Option<&str>> = if leg.levels.is_empty() {
        vec![None]
    } else {
        leg.levels.iter().copied().map(Some).collect()
    };
    for level in levels {
        let one = extract_at(leg, &scratch, &c_path, level);
        all.macros.extend(one.macros);
        all.functions.extend(one.functions);
        all.scanned.extend(one.scanned);
    }
    all
}

/// Extracts every name the emitted unit at `c_path` holds for one leg, read
/// with one optimization flag, or with the leg's flags alone for `None`.
fn extract_at(leg: &Leg, scratch: &ScratchDir, c_path: &Path, level: Option<&str>) -> Extraction {
    let with = |extra: &[&str]| -> Vec<String> {
        level
            .into_iter()
            .chain(extra.iter().copied())
            .map(str::to_owned)
            .collect()
    };
    let run = |extra: &[&str]| {
        let args = with(extra);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        run_compiler(leg, &args, c_path)
    };
    let mut macros = BTreeSet::new();
    for line in run(&["-E", "-dM"]).lines() {
        if let Some(name) = defined_name(line) {
            macros.insert(name.to_owned());
        }
    }
    if let Some(dir) = leg.define_dir {
        define_names_under(Path::new(dir), &mut macros);
    }
    let mut functions = BTreeSet::new();
    if leg.aux_info {
        let report = scratch.path().join("aux-info.txt");
        let report_arg = report.display().to_string();
        run(&["-fsyntax-only", "-aux-info", &report_arg]);
        let text = std::fs::read_to_string(&report)
            .unwrap_or_else(|err| panic!("{}: cannot read the -aux-info report: {err}", leg.name));
        functions = aux_info_functions(&text);
    }
    let mut scanned = BTreeSet::new();
    scan_declarations(&tokenize(&run(&["-E"])), &mut scanned);
    Extraction {
        macros,
        functions,
        scanned,
    }
}

/// True for a name C reserves for the implementation: `__` followed by
/// anything, or `_` followed by an upper-case letter. The emitter escapes
/// those by their shape, so the list never needs them.
fn is_reserved_shape(name: &str) -> bool {
    let bytes = name.as_bytes();
    name.starts_with("__") || (bytes.first() == Some(&b'_') && bytes.get(1).is_some_and(u8::is_ascii_uppercase))
}

/// True when the list, a keyword, or the reserved shape already covers a name,
/// or the name is one of the sample program's own symbols.
fn covered(name: &str, list: &BTreeSet<&str>) -> bool {
    list.contains(name)
        || C_KEYWORDS.contains(&name)
        || is_reserved_shape(name)
        || PROGRAM_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

/// The committed list as a set.
fn committed_list() -> BTreeSet<&'static str> {
    C_NAMES.lines().filter(|line| !line.is_empty()).collect()
}

/// Asserts every name each leg extracts is covered, naming each missing one.
///
/// The failure prints one `missing: <name> (<legs>)` line per name, so the
/// list can be extended by taking the second field of every such line.
fn assert_legs_covered(legs: &[Leg]) {
    let list = committed_list();
    let mut missing: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for leg in legs {
        for name in extract(leg).all() {
            if !covered(&name, &list) {
                missing.entry(name).or_default().push(leg.name);
            }
        }
    }
    if missing.is_empty() {
        return;
    }
    let lines: Vec<String> = missing
        .iter()
        .map(|(name, legs)| format!("missing: {name} ({})", legs.join(", ")))
        .collect();
    panic!(
        "{} header or runtime names are not in compiler/src/codegen/c_names.txt:\n{}",
        missing.len(),
        lines.join("\n")
    );
}

/// Every name the SDK image's legs define or declare is in the committed list.
#[test]
fn should_list_every_header_name_when_any_sdk_leg_preprocesses_the_unit() {
    let mut present = Vec::new();
    for leg in sdk_legs() {
        if tool_available(&leg.compiler) && leg.define_dir.is_none_or(|dir| Path::new(dir).is_dir()) {
            present.push(leg);
        } else {
            eprintln!("skip: {} ({} is absent)", leg.name, leg.compiler);
        }
    }
    assert_legs_covered(&present);
}

/// The declaration scan finds every function GCC's own `-aux-info` report
/// lists for the hosted-amiga unit, the largest and most macro-heavy one, at
/// each optimization level; the scan is the only source of typedefs, objects,
/// and enumerators, so its recall on functions measures how much it can be
/// trusted for those.
#[test]
fn should_find_every_aux_info_function_when_the_scan_reads_the_hosted_amiga_unit() {
    let Some(leg) = sdk_legs().into_iter().find(|leg| leg.name == "hosted-amiga") else {
        panic!("the hosted-amiga leg is missing from the leg table");
    };
    if !tool_available(&leg.compiler) {
        eprintln!("skip: {} is absent", leg.compiler);
        return;
    }
    let scratch = ScratchDir::new("recall");
    let c_path = emit_sample(&scratch, leg.profile);
    for level in leg.levels {
        let extraction = extract_at(&leg, &scratch, &c_path, Some(level));
        assert!(
            extraction.functions.len() > 1000,
            "{level}: the -aux-info report lists only {} functions; the report or its parser is broken",
            extraction.functions.len()
        );
        // A reserved-shaped name is escaped by its shape and never listed, and
        // the release line's inline libnix helpers declare one at block scope
        // (`__bcopz`), which declares nothing at file scope; only the names the
        // list can need are compared.
        let missed: Vec<&String> = extraction
            .functions
            .iter()
            .filter(|name| !is_reserved_shape(name) && !extraction.scanned.contains(*name))
            .collect();
        assert!(
            missed.is_empty(),
            "{level}: the declaration scan misses {} functions -aux-info reports: {missed:?}",
            missed.len()
        );
    }
}

// ---------------------------------------------------------------------------
// The list's own shape
// ---------------------------------------------------------------------------

/// The list is one ASCII identifier per line, in byte order, with no
/// duplicate, keyword, or reserved-shaped entry: the emitter probes it by
/// binary search, and the other two classes are escaped without it.
#[test]
fn should_hold_sorted_unique_plain_identifiers_when_the_list_is_read() {
    let lines: Vec<&str> = C_NAMES.lines().collect();
    assert!(lines.len() > 5000, "the list holds only {} names", lines.len());
    for pair in lines.windows(2) {
        assert!(
            pair[0].as_bytes() < pair[1].as_bytes(),
            "the list is not strictly byte-sorted at {:?} then {:?}",
            pair[0],
            pair[1]
        );
    }
    for name in &lines {
        let identifier = !name.is_empty()
            && !name.starts_with(|ch: char| ch.is_ascii_digit())
            && name.bytes().all(|byte| byte == b'_' || byte.is_ascii_alphanumeric());
        assert!(identifier, "the list entry {name:?} is not a C identifier");
        assert!(!C_KEYWORDS.contains(name), "the list repeats the keyword {name:?}");
        assert!(!is_reserved_shape(name), "the list repeats the reserved name {name:?}");
    }
}

/// No keyword or listed name is the escaped spelling of an escaped name, so
/// an escaped source name can never land on a header name.
#[test]
fn should_never_list_an_escaped_spelling_when_its_root_is_escaped() {
    for entry in C_NAMES.lines().chain(C_KEYWORDS.iter().copied()) {
        let Some(root) = entry.strip_suffix("_0") else {
            continue;
        };
        assert_eq!(
            clc::codegen::escape_c_name(root),
            root,
            "`{entry}` is listed, but it is also the escaped spelling of `{root}`"
        );
    }
}

// ---------------------------------------------------------------------------
// The escape function
// ---------------------------------------------------------------------------

/// Names in no class keep their spelling; keywords, reserved identifiers,
/// listed names, and any name whose `_<digits>`-stripped root is one of
/// those gain `_0`; the empty name is `_0`.
#[test]
fn should_append_one_stable_suffix_when_a_name_or_its_root_is_escaped() {
    let cases = [
        ("count", "count"),
        ("bufferSize", "bufferSize"),
        ("height", "height"),
        ("Gps", "Gps"),
        ("DAY", "DAY"),
        ("Point", "Point_0"),
        ("int", "int_0"),
        ("restrict", "restrict_0"),
        ("_Bool", "_Bool_0"),
        ("bool", "bool_0"),
        ("int_0", "int_0_0"),
        ("int_0_0", "int_0_0_0"),
        ("EOF", "EOF_0"),
        ("EOF_2", "EOF_2_0"),
        ("EOF_2_10", "EOF_2_10_0"),
        ("DAY_1", "DAY_1_0"),
        ("memcpy", "memcpy_0"),
        ("textPen", "textPen_0"),
        ("__LINE__", "__LINE___0"),
        ("_Pragma", "_Pragma_0"),
        ("__cl_owned0", "__cl_owned0_0"),
        ("", "_0"),
    ];
    for (name, expected) in cases {
        assert_eq!(clc::codegen::escape_c_name(name), expected, "escape of {name:?}");
    }
}

/// The escape is injective and never produces a keyword or a listed name,
/// over every listed name, keyword, and kept name together with their
/// counter- and escape-shaped variants.
#[test]
fn should_give_distinct_unlisted_spellings_when_many_names_are_escaped() {
    let list = committed_list();
    let mut inputs: BTreeSet<String> = BTreeSet::new();
    let roots = C_NAMES
        .lines()
        .chain(C_KEYWORDS.iter().copied())
        .chain(["count", "x", "Gps", "DAY", "__LINE__", "_Pragma", "__cl_tmp0"]);
    for root in roots {
        for suffix in ["", "_0", "_1", "_2", "_0_0", "_0_1", "_10"] {
            inputs.insert(format!("{root}{suffix}"));
        }
    }
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for input in &inputs {
        let spelled = clc::codegen::escape_c_name(input);
        assert!(
            !C_KEYWORDS.contains(&spelled.as_str()) && !list.contains(spelled.as_str()),
            "`{input}` is spelled `{spelled}`, which is a keyword or a listed name"
        );
        if let Some(previous) = seen.insert(spelled.clone(), input.clone()) {
            panic!("`{previous}` and `{input}` are both spelled `{spelled}`");
        }
    }
}

// ---------------------------------------------------------------------------
// Emitted spellings
// ---------------------------------------------------------------------------

/// Emits `source` for `profile` through the compiler library and returns the
/// unit's text.
fn emit_program(tag: &str, profile: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let path = scratch.path().join("main.clo");
    std::fs::write(&path, source).expect("write the program");
    let c_path = scratch.path().join("out.c");
    let status = clc::driver::run(vec![
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        path.display().to_string(),
    ]);
    assert_eq!(status, 0, "{tag}: the program must compile for {profile}");
    std::fs::read_to_string(&c_path).expect("read the emitted C")
}

/// Builds C text with the strict host flags, runs it, and returns stdout.
fn run_host_c(tag: &str, c: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("out.c");
    std::fs::write(&c_path, c).expect("write the C");
    let program = scratch.path().join("program");
    let built = Command::new("cc")
        .args(["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror", "-o"])
        .arg(&program)
        .arg(&c_path)
        .output()
        .expect("run cc");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let run = Command::new("timeout").arg("30").arg(&program).output().expect("run the program");
    assert_eq!(run.status.code(), Some(0), "{tag}: the program failed");
    String::from_utf8_lossy(&run.stdout).into_owned()
}

/// Builds hosted-amiga C with the strict m68k debug line and runs it under
/// `vamos`; returns `None` with a `skip:` note when the tools are absent.
fn run_m68k_c(tag: &str, c: &str, crt: &str) -> Option<String> {
    if !tool_available("m68k-amigaos-gcc") || !on_path("vamos") {
        eprintln!("skip: {tag}: m68k-amigaos-gcc or vamos is absent");
        return None;
    }
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("out.c");
    std::fs::write(&c_path, c).expect("write the C");
    let program = scratch.path().join("program");
    let built = Command::new("m68k-amigaos-gcc")
        .arg("-std=c99")
        .arg(format!("-mcrt={crt}"))
        .args(["-m68000", "-fwrapv", "-O0", "-g", "-Wall", "-Wextra", "-Werror", "-o"])
        .arg(&program)
        .arg(&c_path)
        .args(["-lamiga", "-lm"])
        .output()
        .expect("run m68k-amigaos-gcc");
    assert!(
        built.status.success(),
        "{tag}: m68k-amigaos-gcc rejected the emitted C:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let run = Command::new("timeout")
        .arg("120")
        .arg("vamos")
        .arg(&program)
        .current_dir(scratch.path())
        .output()
        .expect("run vamos");
    assert_eq!(run.status.code(), Some(0), "{tag}: the program failed under vamos");
    Some(String::from_utf8_lossy(&run.stdout).replace('\r', ""))
}

/// The identifier tokens of a C text.
fn identifiers(c: &str) -> BTreeSet<&str> {
    c.split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .filter(|token| token.starts_with(|ch: char| ch == '_' || ch.is_ascii_alphabetic()))
        .collect()
}

/// The emitted definition of the function whose C symbol is `symbol`: from its
/// header line to the closing brace that ends it.
fn function_text<'c>(c: &'c str, symbol: &str) -> &'c str {
    find_function_text(c, symbol).unwrap_or_else(|| panic!("no definition of {symbol} in the emitted C"))
}

/// [`function_text`], or `None` when the unit defines no such function.
fn find_function_text<'c>(c: &'c str, symbol: &str) -> Option<&'c str> {
    let header = c
        .lines()
        .position(|line| line.contains(&format!("{symbol}(")) && !line.trim_end().ends_with(';'))?;
    let start = c.lines().take(header).map(|line| line.len() + 1).sum::<usize>();
    let rest = &c[start..];
    let end = rest.find("\n}\n").map_or(rest.len(), |end| end + 3);
    Some(&rest[..end])
}

/// The definitions of every lifted lambda in the unit, in site order.
fn lifted_texts(c: &str) -> Vec<&str> {
    (0..64)
        .filter_map(|site| find_function_text(c, &format!("cl_lambda_{site}")))
        .collect()
}

/// The compiler's own spellings keep their form: the hoisted result
/// `__cl_ret`, a temporary `__cl_tmp<N>`, a lambda's `__env`, and a generated
/// owner `__cl_owned0`, while a source local spelled `__cl_owned0` is escaped
/// to `__cl_owned0_0` and keeps its own value. The generated owner is read in
/// a function of its own, where nothing else holds its spelling.
#[test]
fn should_keep_generated_spellings_when_a_source_local_shares_one() {
    let source = r#"module Generated;

import { println } from std.io;
import { String, stringOf } from std.text;

func use(borrow s: String): i32 {
    return i32(len(s));
}

func viaOwner(): i32 {
    return use(stringOf("w"));
}

func pick(flag: bool): i32[2] {
    let pair: i32[2] = [1, 2];
    if (flag) {
        return pair;
    }
    return [3, 4];
}

func main(): i32 {
    let __cl_owned0: String = stringOf("qq");
    let n: i32 = viaOwner();
    let base: i32 = 5;
    let r: i32 = ([base] (x: i32) => x + base)(1);
    let p: i32[2] = pick(true);
    println($"{n} {i32(len(__cl_owned0))} {r} {p[1]}");
    return 0;
}
"#;
    let c = emit_program("generated", "host", source);
    let main = function_text(&c, "cl_Generated_main");
    let main_identifiers = identifiers(main);
    assert!(
        main_identifiers.contains("__cl_owned0_0") && !main_identifiers.contains("__cl_owned0"),
        "the source local `__cl_owned0` must be escaped in main:\n{main}"
    );
    let owner = function_text(&c, "cl_Generated_viaOwner");
    assert!(
        identifiers(owner).contains("__cl_owned0"),
        "the generated owner keeps the spelling `__cl_owned0`:\n{owner}"
    );
    assert!(
        main_identifiers.iter().any(|name| name.starts_with("__cl_tmp")),
        "a temporary keeps its `__cl_tmp<N>` spelling:\n{main}"
    );
    assert!(identifiers(&c).contains("__env"), "the lambda keeps its `__env` parameter");
    assert!(
        identifiers(function_text(&c, "cl_Generated_pick")).contains("__cl_ret"),
        "the fixed-array result keeps its `__cl_ret` spelling"
    );
    assert_eq!(run_host_c("generated", &c), "1 2 6 2\n");
}

/// Every local and parameter of a function is given its spelling in the
/// local-name table: each escaped source name is read, written, compounded,
/// addressed, passed, looped on, and returned, and the function's text holds
/// only the escaped spellings, never one a fallback would leave raw. The
/// locals of a capturing and of a capture-less lambda body take the same
/// route through their lifted functions.
#[test]
fn should_spell_every_use_through_the_table_when_names_are_escaped() {
    let source = r#"module Uses;

import { println } from std.io;

func bump(textPen: i32): i32 {
    return textPen + 1;
}

func work(EOF: i32, errno: i32): i32 {
    let SysBase: i32 = EOF + errno;
    SysBase += bump(errno);
    let detailPen: raw i32 = unsafe &SysBase;
    unsafe {
        *detailPen = *detailPen + 1;
    }
    for (let textPen: i32 = 0; textPen < 3; textPen += 1) {
        SysBase = SysBase + textPen;
    }
    while (SysBase > 100) {
        SysBase = SysBase - 100;
    }
    if (SysBase == errno) {
        return 0;
    }
    return SysBase;
}

func main(): i32 {
    let total: i32 = work(1, 2);
    let k: i32 = 3;
    let viaCapture: i32 = ([k] (x: i32) => {
        let EOF: i32 = x + k;
        let errno: i32 = 1;
        return EOF + errno;
    })(2);
    let plain: func(i32): i32 = (x: i32) => {
        let restrict: i32 = x * 2;
        let textPen: i32 = 1;
        return restrict + textPen;
    };
    let viaPlain: i32 = plain(5);
    println($"{total} {viaCapture} {viaPlain}");
    return 0;
}
"#;
    let c = emit_program("uses", "host", source);
    let assert_escaped = |label: &str, text: &str, names: &[&str]| {
        let found = identifiers(text);
        for name in names {
            assert!(
                !found.contains(name),
                "{label} spells the source name `{name}` raw:\n{text}"
            );
            assert!(
                found.contains(format!("{name}_0").as_str()),
                "{label} never spells `{name}` escaped:\n{text}"
            );
        }
    };
    assert_escaped(
        "cl_Uses_work",
        function_text(&c, "cl_Uses_work"),
        &["EOF", "errno", "SysBase", "detailPen", "textPen"],
    );
    assert_escaped("cl_Uses_bump", function_text(&c, "cl_Uses_bump"), &["textPen"]);
    let lifted = lifted_texts(&c);
    assert_eq!(lifted.len(), 2, "the two lambdas are lifted into two functions");
    let capturing = lifted
        .iter()
        .find(|text| text.contains("__env"))
        .expect("the capturing lambda's function takes its environment");
    let capture_less = lifted
        .iter()
        .find(|text| !text.contains("__env"))
        .expect("the capture-less lambda's function has no environment");
    assert_escaped("the capturing lambda", capturing, &["EOF", "errno"]);
    assert_escaped("the capture-less lambda", capture_less, &["restrict", "textPen"]);
    assert_eq!(run_host_c("uses", &c), "10 6 11\n");
    if let Some(output) = run_m68k_c("uses", &emit_program("uses-amiga", "hosted-amiga", source), "nix20") {
        assert_eq!(output, "10 6 11\n");
    }
}

/// A name shadowed twice takes a distinct escaped spelling per binding, none
/// of them a listed name, and the program builds and runs on host and on
/// hosted-amiga, where the listed names are macros.
#[test]
fn should_spell_each_shadow_apart_when_an_escaped_name_is_shadowed_twice() {
    let source = r#"module Shadow;

import { println } from std.io;

func main(): i32 {
    let DAY: i32 = 1;
    let EOF: i32 = 10;
    let total: i32 = DAY + EOF;
    {
        let DAY: i32 = 2;
        let EOF: i32 = 20;
        total += DAY + EOF;
        {
            let DAY: i32 = 3;
            let EOF: i32 = 30;
            total += DAY + EOF;
        }
    }
    println($"{total}");
    return 0;
}
"#;
    let list = committed_list();
    for profile in ["host", "hosted-amiga"] {
        let c = emit_program(&format!("shadow-{profile}"), profile, source);
        let main = function_text(&c, "cl_Shadow_main");
        let spellings: Vec<&str> = identifiers(main)
            .into_iter()
            .filter(|name| name.starts_with("DAY") || name.starts_with("EOF"))
            .collect();
        assert_eq!(
            spellings.len(),
            6,
            "{profile}: three bindings of each name take six spellings: {spellings:?}"
        );
        for name in &spellings {
            assert!(!list.contains(name), "{profile}: `{name}` is a listed name:\n{main}");
        }
    }
    let c = emit_program("shadow-run", "host", source);
    assert_eq!(run_host_c("shadow", &c), "66\n");
    if let Some(output) = run_m68k_c("shadow", &emit_program("shadow-amiga", "hosted-amiga", source), "nix20") {
        assert_eq!(output, "66\n");
    }
}

/// Every name the rc24 toolchain's hosted legs define or declare is in the
/// committed list. The case runs only with `CLORRAINE_RC24_ROOT` set, and a
/// set but unusable root fails rather than skips.
#[test]
fn should_list_every_rc24_header_name_when_the_rc24_root_is_given() {
    let Some(root) = std::env::var_os("CLORRAINE_RC24_ROOT") else {
        eprintln!("skip: CLORRAINE_RC24_ROOT is not set");
        return;
    };
    let legs = rc24_legs(Path::new(&root));
    for leg in &legs {
        assert!(
            tool_available(&leg.compiler),
            "CLORRAINE_RC24_ROOT is set, but {} does not run",
            leg.compiler
        );
    }
    assert_legs_covered(&legs);
}
