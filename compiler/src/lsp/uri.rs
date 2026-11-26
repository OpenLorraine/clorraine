//! Document URI conversion.
//!
//! The protocol addresses documents by URI. A file URI maps to a path on this
//! platform; an `untitled:` URI names an unsaved buffer and never touches the
//! file system. Everything else is rejected, so a client cannot ask the
//! workspace index to read an arbitrary scheme.

use std::path::{Path, PathBuf};

use crate::clorraine::platform::{self, HostOs};

/// A document address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentUri {
    /// A file on disk.
    File(PathBuf),
    /// An unsaved editor buffer.
    Untitled(String),
}

impl DocumentUri {
    /// Returns the logical path used in a [`crate::span::SourceMap`].
    ///
    /// A file keeps its absolute path spelling; an untitled buffer keeps the
    /// URI so its diagnostics can be routed back to the editor.
    pub fn key(&self) -> String {
        match self {
            DocumentUri::File(path) => normalize(&path.display().to_string()),
            DocumentUri::Untitled(uri) => uri.clone(),
        }
    }

    /// Returns the protocol URI spelling.
    pub fn to_uri(&self) -> String {
        match self {
            DocumentUri::File(path) => from_path(path),
            DocumentUri::Untitled(uri) => uri.clone(),
        }
    }

    /// Returns the file path, when this is a file URI.
    pub fn path(&self) -> Option<&Path> {
        match self {
            DocumentUri::File(path) => Some(path),
            DocumentUri::Untitled(_) => None,
        }
    }
}

/// Parses a `file:` or `untitled:` URI.
pub fn parse(uri: &str) -> Result<DocumentUri, String> {
    parse_for(HostOs::current(), uri)
}

/// Parses a `file:` or `untitled:` URI under the given host family's rules.
///
/// On Windows a file URI names a drive path: `file:///c%3A/a/b.clo` and
/// `file:///C:/a/b.clo` both become `C:\a\b.clo`, with an uppercase drive and
/// backslashes, the one spelling every key and reply uses. A URI with a host
/// (a network share) is refused there, apart from `localhost`.
///
/// # Errors
///
/// A message for an unsupported scheme, a remote host, a malformed escape, or
/// (on Windows) a path without a drive.
///
/// ```
/// use clc::clorraine::platform::HostOs;
/// use clc::lsp::uri::{parse_for, DocumentUri};
/// let uri = parse_for(HostOs::Windows, "file:///c%3A/x.clo").unwrap();
/// assert_eq!(uri, DocumentUri::File(r"C:\x.clo".into()));
/// ```
pub fn parse_for(os: HostOs, uri: &str) -> Result<DocumentUri, String> {
    if os == HostOs::Windows {
        return parse_windows(uri);
    }
    if uri.starts_with("untitled:") {
        return Ok(DocumentUri::Untitled(uri.to_owned()));
    }
    let Some(rest) = uri.strip_prefix("file://") else {
        return Err(format!("unsupported document URI scheme: {uri}"));
    };
    // `file:///path`, `file://localhost/path`, and `file://host/path` are the
    // three shapes a client may send. A remote host cannot be resolved here.
    let path_part = if let Some(path) = rest.strip_prefix('/') {
        format!("/{path}")
    } else if let Some(path) = rest.strip_prefix("localhost/") {
        format!("/{path}")
    } else {
        return Err(format!("file URI names a remote host: {uri}"));
    };
    let decoded = percent_decode(&path_part)?;
    if decoded.is_empty() {
        return Err("file URI has an empty path".to_owned());
    }
    Ok(DocumentUri::File(PathBuf::from(decoded)))
}

/// Parses a URI with the Windows rules of [`parse_for`].
fn parse_windows(uri: &str) -> Result<DocumentUri, String> {
    if uri.starts_with("untitled:") {
        return Ok(DocumentUri::Untitled(uri.to_owned()));
    }
    let Some(rest) = uri.strip_prefix("file://") else {
        return Err(format!("unsupported document URI scheme: {uri}"));
    };
    let path_part = if let Some(path) = rest.strip_prefix('/') {
        path
    } else if let Some(path) = rest.strip_prefix("localhost/") {
        path
    } else {
        return Err(format!("network-share paths are not supported: {uri}"));
    };
    let decoded = percent_decode(path_part)?;
    let bytes = decoded.as_bytes();
    if bytes.len() < 2 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return Err(format!("file URI has no drive letter: {uri}"));
    }
    Ok(DocumentUri::File(PathBuf::from(normalize_for(HostOs::Windows, &decoded.replace('/', "\\")))))
}

/// Renders an absolute path as a `file:` URI.
pub fn from_path(path: &Path) -> String {
    from_path_for(HostOs::current(), &path.display().to_string())
}

/// Renders an absolute path as a `file:` URI under the given host family's
/// rules.
///
/// Unix paths percent-encode every byte except the unreserved characters and
/// `/`. A Windows drive path takes the shape VS Code itself sends:
/// `C:\a b\x.clo` becomes `file:///c%3A/a%20b/x.clo`, with a lowercase drive,
/// an encoded colon, and forward slashes.
///
/// ```
/// use clc::clorraine::platform::HostOs;
/// use clc::lsp::uri::from_path_for;
/// assert_eq!(from_path_for(HostOs::Windows, r"C:\a b\x.clo"), "file:///c%3A/a%20b/x.clo");
/// ```
pub fn from_path_for(os: HostOs, path: &str) -> String {
    let mut encoded = String::from("file://");
    let text = match os {
        HostOs::Unix => path.to_owned(),
        HostOs::Windows => {
            let normal = normalize_for(os, path).replace('\\', "/");
            let bytes = normal.as_bytes();
            if bytes.len() >= 2 && bytes[1] == b':' {
                encoded.push('/');
                encoded.push(char::from(bytes[0].to_ascii_lowercase()));
                encoded.push_str("%3A");
                normal[2..].to_owned()
            } else {
                normal
            }
        }
    };
    for byte in text.bytes() {
        if is_unreserved(byte) || byte == b'/' {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// The one spelling of a path that every document key, index key, and
/// source-selection reply uses on this host.
pub fn normalize(path: &str) -> String {
    normalize_for(HostOs::current(), path)
}

/// [`normalize`] under the given host family's rules.
///
/// Windows drops the verbatim `\\?\` prefix that resolving a path adds and
/// uppercases the drive letter, so the editor's `c:\…` and the server's
/// `\\?\C:\…` name the same key. Unix paths are unchanged.
///
/// ```
/// use clc::clorraine::platform::HostOs;
/// use clc::lsp::uri::normalize_for;
/// assert_eq!(normalize_for(HostOs::Windows, r"\\?\c:\x.clo"), r"C:\x.clo");
/// ```
pub fn normalize_for(os: HostOs, path: &str) -> String {
    if os == HostOs::Unix {
        return path.to_owned();
    }
    let shown = platform::display_path(os, path);
    let bytes = shown.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_lowercase() && bytes[1] == b':' {
        let mut upper = String::with_capacity(shown.len());
        upper.push(char::from(bytes[0].to_ascii_uppercase()));
        upper.push_str(&shown[1..]);
        return upper;
    }
    shown
}

/// True for the URI unreserved characters.
fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// Decodes `%XX` escapes into a UTF-8 string.
fn percent_decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("truncated percent escape in URI".to_owned());
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                .map_err(|_| "invalid percent escape".to_owned())?;
            let value = u8::from_str_radix(hex, 16)
                .map_err(|_| format!("invalid percent escape '%{hex}'"))?;
            out.push(value);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "URI path is not valid UTF-8".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_round_trip_a_path_with_spaces_and_unicode() {
        let path = PathBuf::from("/tmp/a b/cé.clo");
        let uri = from_path(&path);
        assert!(uri.starts_with("file:///tmp/a%20b/"));
        assert_eq!(parse(&uri), Ok(DocumentUri::File(path)));
    }

    #[test]
    fn should_accept_localhost_and_reject_a_remote_host() {
        assert_eq!(
            parse("file://localhost/tmp/a.clo"),
            Ok(DocumentUri::File(PathBuf::from("/tmp/a.clo")))
        );
        assert!(parse("file://example.com/tmp/a.clo").is_err());
        assert!(parse("https://example.com/a.clo").is_err());
    }

    #[test]
    fn should_keep_an_untitled_uri_opaque() {
        let uri = parse("untitled:Untitled-1").expect("untitled");
        assert_eq!(uri.key(), "untitled:Untitled-1");
        assert!(uri.path().is_none());
    }

    #[test]
    fn should_reject_a_truncated_escape() {
        assert!(parse("file:///tmp/a%2.clo").is_err());
        assert!(parse("file:///tmp/a%zz.clo").is_err());
    }
}
