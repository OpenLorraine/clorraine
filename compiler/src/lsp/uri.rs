//! Document URI conversion.
//!
//! The protocol addresses documents by URI. A file URI maps to a path on this
//! platform; an `untitled:` URI names an unsaved buffer and never touches the
//! file system. Everything else is rejected, so a client cannot ask the
//! workspace index to read an arbitrary scheme.

use std::path::{Path, PathBuf};

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
            DocumentUri::File(path) => path.display().to_string(),
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

/// Renders an absolute path as a `file:` URI.
pub fn from_path(path: &Path) -> String {
    let text = path.display().to_string();
    let mut encoded = String::from("file://");
    for byte in text.bytes() {
        if is_unreserved(byte) || byte == b'/' {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
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
