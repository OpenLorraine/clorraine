//! Bounded native-installation receipts, using the existing public JSON model.
//!
//! These receipts identify an assembled SDK and its companion wheel cohort.
//! They are not an installer completion record or an authentication mechanism
//! against a local user who can replace the installation itself.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

use crate::lsp::json::Json;

/// Maximum size of an individual native receipt or venv configuration file.
const METADATA_LIMIT: u64 = 16 * 1024 * 1024;

/// A supported system-Python cohort, rather than an arbitrary interpreter tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PythonAbi {
    /// Ubuntu 22.04's CPython 3.10.
    Cp310,
    /// Ubuntu 24.04's CPython 3.12.
    Cp312,
}

impl PythonAbi {
    /// Returns the literal cohort recorded in the copied wheel receipt.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Cp310 => "cp310",
            Self::Cp312 => "cp312",
        }
    }
}

/// Assembly identities that must agree with preparation and wheel receipts.
pub(super) struct Installation {
    /// Exact support-lock bytes consumed during assembly.
    pub(super) lock: String,
    /// Exact user-provided NDK archive used in preparation.
    pub(super) ndk: String,
    /// Deterministic prepared-header inventory identity.
    pub(super) inventory: String,
    /// Cohort selected when the private venv was assembled.
    pub(super) abi: PythonAbi,
}

/// Reads an ordinary, bounded UTF-8 file without accepting metadata aliases.
///
/// Linux opens with no-follow and nonblocking flags before checking the opened
/// descriptor. That closes the gap between a path check and opening a swapped
/// symlink or FIFO. Other hosts cannot select a native Linux SDK.
pub(super) fn ordinary_text(path: &Path) -> Result<String, String> {
    let status = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !status.is_file() {
        return Err("expected an ordinary file, not a link or special file".to_owned());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // O_NOFOLLOW and O_NONBLOCK keep replacement links/FIFOs from being
        // followed or blocking before the descriptor's type can be checked.
        const SAFE_OPEN_FLAGS: i32 = 0x20000 | 0x800;
        options.custom_flags(SAFE_OPEN_FLAGS);
    }
    let file = options.open(path).map_err(|error| error.to_string())?;
    if !file.metadata().map_err(|error| error.to_string())?.is_file() {
        return Err("opened metadata is not an ordinary file".to_owned());
    }
    let mut bytes = Vec::new();
    file.take(METADATA_LIMIT + 1).read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > METADATA_LIMIT {
        return Err("metadata exceeds 16 MiB".to_owned());
    }
    String::from_utf8(bytes).map_err(|_| "metadata is not valid UTF-8".to_owned())
}

/// Validates the exact six-field descriptor before any selected tool executes.
pub(super) fn installation(path: &Path) -> Result<Installation, String> {
    let json = read_json(path)?;
    let fields = object(&json, &[
        "schema", "core_version", "support_lock_sha256", "ndk_sha256",
        "prepared_inventory_sha256", "python_abi",
    ])?;
    schema(fields)?;
    let version = text(fields, "core_version")?;
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|part| {
        part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit())
    }) {
        return Err("core_version must have three decimal components".to_owned());
    }
    let abi = match text(fields, "python_abi")? {
        "cp310" => PythonAbi::Cp310,
        "cp312" => PythonAbi::Cp312,
        _ => return Err("python_abi must be cp310 or cp312".to_owned()),
    };
    Ok(Installation {
        lock: hash(fields, "support_lock_sha256")?.to_owned(),
        ndk: hash(fields, "ndk_sha256")?.to_owned(),
        inventory: hash(fields, "prepared_inventory_sha256")?.to_owned(),
        abi,
    })
}

/// Checks that preparation used the same support, NDK and inventory identities.
pub(super) fn preparation(path: &Path, selected: &Installation) -> Result<(), String> {
    let json = read_json(path)?;
    let fields = object(&json, &["schema", "lock_sha256", "ndk_sha256", "inventory_sha256"])?;
    schema(fields)?;
    if hash(fields, "lock_sha256")? != selected.lock
        || hash(fields, "ndk_sha256")? != selected.ndk
        || hash(fields, "inventory_sha256")? != selected.inventory
    {
        return Err("preparation receipt does not match the installation descriptor".to_owned());
    }
    Ok(())
}

/// Validates the complete copied wheel receipt and its installation pairing.
///
/// The receipt describes the source wheels, not files inside the venv. Archive
/// authentication and installed-package checks belong to assembly; starting a
/// program must not depend on keeping a second wheel cache in the venv.
pub(super) fn wheels(path: &Path, selected: &Installation) -> Result<(), String> {
    let json = read_json(path)?;
    let fields = object(&json, &["schema", "lock_sha256", "abi", "packages"])?;
    schema(fields)?;
    if hash(fields, "lock_sha256")? != selected.lock
        || text(fields, "abi")? != selected.abi.name()
    {
        return Err("wheel ABI/support lock does not match the selected installation".to_owned());
    }
    let packages = fields.get("packages").and_then(Json::as_object)
        .ok_or_else(|| "packages must be an object".to_owned())?;
    for (name, package) in packages {
        if !identifier(name) || name.contains('.') || name.contains('_') {
            return Err("wheel package names must be normalized lowercase identifiers".to_owned());
        }
        let record = object(package, &[
            "version", "source", "purpose", "filename", "bytes", "sha256", "tags",
        ])?;
        let version = text(record, "version")?;
        if !version.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || b".!+_~:-".contains(&byte)
        }) {
            return Err(format!("{name}: invalid package version"));
        }
        if !identifier(text(record, "source")?) {
            return Err(format!("{name}: invalid source identity"));
        }
        if !matches!(text(record, "purpose")?, "runtime" | "bootstrap") {
            return Err(format!("{name}: invalid wheel purpose"));
        }
        let filename = text(record, "filename")?;
        if !relative(filename) || !filename.ends_with(".whl") {
            return Err(format!("{name}: unsafe wheel filename"));
        }
        if record.get("bytes").and_then(Json::as_i64).is_none_or(|bytes| bytes <= 0) {
            return Err(format!("{name}: bytes must be a positive integer"));
        }
        hash(record, "sha256")?;
        let tags = record.get("tags").and_then(Json::as_array)
            .ok_or_else(|| format!("{name}: tags must be an array"))?;
        let mut seen = BTreeSet::new();
        if tags.is_empty() || tags.iter().any(|tag| {
            tag.as_str().is_none_or(|tag| !plain_text(tag) || !seen.insert(tag))
        }) {
            return Err(format!("{name}: tags must be nonempty unique strings"));
        }
    }
    for (name, purpose) in [
        ("amitools", "runtime"), ("machine68k", "runtime"), ("greenlet", "runtime"),
        ("pysdl2", "runtime"), ("pip", "bootstrap"),
    ] {
        if packages.get(name).and_then(|package| package.get("purpose"))
            .and_then(Json::as_str) != Some(purpose)
        {
            return Err(format!("{name}: missing required {purpose} wheel receipt"));
        }
    }
    Ok(())
}

/// Reads JSON with duplicate-key and canonical integer-token checks.
fn read_json(path: &Path) -> Result<Json, String> {
    let text = ordinary_text(path)?;
    let value = Json::parse(&text).map_err(|error| {
        format!("invalid JSON at byte {}: {}", error.offset, error.message)
    })?;
    strict_tokens(&text)?;
    Ok(value)
}

/// Requires the exact record fields before indexing any of their values.
fn object<'a>(value: &'a Json, names: &[&str]) -> Result<&'a BTreeMap<String, Json>, String> {
    let fields = value.as_object().ok_or_else(|| "expected a JSON object".to_owned())?;
    if fields.len() != names.len() || names.iter().any(|name| !fields.contains_key(*name)) {
        return Err(format!("missing or unknown fields; expected {}", names.join(", ")));
    }
    Ok(fields)
}

/// Distinguishes integer schema one from booleans and other versions.
fn schema(fields: &BTreeMap<String, Json>) -> Result<(), String> {
    if fields.get("schema") == Some(&Json::Number(1)) {
        Ok(())
    } else {
        Err("schema must be integer 1".to_owned())
    }
}

/// Requires nonempty text without terminal or filename control characters.
fn plain_text(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

/// Retrieves one typed text field without coercing numbers or nulls.
fn text<'a>(fields: &'a BTreeMap<String, Json>, name: &str) -> Result<&'a str, String> {
    fields.get(name).and_then(Json::as_str).filter(|value| plain_text(value))
        .ok_or_else(|| format!("{name} must be nonempty text without controls"))
}

/// Requires a complete lowercase SHA-256 identity, not a prefix or token.
fn hash<'a>(fields: &'a BTreeMap<String, Json>, name: &str) -> Result<&'a str, String> {
    let value = text(fields, name)?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(format!("{name} must be a lowercase SHA-256"));
    }
    Ok(value)
}

/// Applies the existing source/helper identifier grammar.
fn identifier(value: &str) -> bool {
    value.bytes().next().is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && value.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte))
}

/// Keeps a receipt filename canonical and relative; it is never executable code.
fn relative(value: &str) -> bool {
    !value.starts_with('/') && !value.contains(['\\', '!'])
        && value.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

/// Adds strict receipt rules without changing the language server's parser.
///
/// The public parser has already checked syntax, strings and nesting bounds.
/// This scan retains each object's original keys before its map can hide
/// duplicates, and rejects integer spellings with leading zeroes. A string
/// followed by a colon can only be a key in that already-validated JSON.
fn strict_tokens(text: &str) -> Result<(), String> {
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut objects: Vec<Option<BTreeSet<String>>> = Vec::new();
    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                objects.push(Some(BTreeSet::new()));
                index += 1;
            }
            b'[' => {
                objects.push(None);
                index += 1;
            }
            b'}' | b']' => {
                objects.pop();
                index += 1;
            }
            b'"' => {
                let start = index;
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    index += if bytes[index] == b'\\' { 2 } else { 1 };
                }
                index += 1;
                let mut after = index;
                while bytes.get(after).is_some_and(u8::is_ascii_whitespace) {
                    after += 1;
                }
                if bytes.get(after) == Some(&b':') {
                    let key = Json::parse(&text[start..index])
                        .map_err(|_| "invalid object key".to_owned())?;
                    let key = key.as_str().ok_or_else(|| "invalid object key".to_owned())?;
                    let keys = objects.last_mut().and_then(Option::as_mut)
                        .ok_or_else(|| "object key outside an object".to_owned())?;
                    if !keys.insert(key.to_owned()) {
                        return Err("duplicate JSON key".to_owned());
                    }
                }
            }
            b'-' | b'0'..=b'9' => {
                if bytes[index] == b'-' {
                    index += 1;
                }
                let first = index;
                while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                    index += 1;
                }
                if bytes.get(first) == Some(&b'0') && index - first > 1 {
                    return Err("JSON integers cannot have leading zeroes".to_owned());
                }
            }
            _ => index += 1,
        }
    }
    Ok(())
}
