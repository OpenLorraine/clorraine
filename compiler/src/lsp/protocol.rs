//! LSP message framing over a byte stream.
//!
//! The language server protocol puts one JSON value behind a
//! `Content-Length`-prefixed header block. The reader accepts header names in
//! any case, ignores headers it does not know, and caps a body at eight
//! mebibytes so a hostile input cannot exhaust memory. The writer always emits
//! `\r\n` line endings and a UTF-8 content length in bytes.

use std::io::{self, BufRead, Write};

/// Largest body the server accepts.
pub const MAX_BODY: usize = 8 * 1024 * 1024;

/// A framing failure.
#[derive(Debug)]
pub enum FrameError {
    /// The underlying stream failed.
    Io(io::Error),
    /// The stream ended before a message started.
    Eof,
    /// A header block had no `Content-Length`.
    MissingLength,
    /// `Content-Length` was not a positive integer.
    BadLength,
    /// The body exceeded [`MAX_BODY`].
    TooLarge,
    /// The body was not valid UTF-8.
    BadUtf8,
}

impl From<io::Error> for FrameError {
    fn from(error: io::Error) -> Self {
        FrameError::Io(error)
    }
}

/// Reads framed messages.
pub struct FrameReader<R: BufRead> {
    reader: R,
}

impl<R: BufRead> FrameReader<R> {
    /// Wraps a buffered reader.
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Reads one message.
    ///
    /// Returns `Ok(None)` on a clean end of stream before a header block.
    pub fn read(&mut self) -> Result<Option<String>, FrameError> {
        let mut length: Option<usize> = None;
        let mut saw_header = false;
        loop {
            let mut line = String::new();
            let read = self.reader.read_line(&mut line)?;
            if read == 0 {
                return if saw_header {
                    Err(FrameError::Eof)
                } else {
                    Ok(None)
                };
            }
            saw_header = true;
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                break;
            }
            let Some((name, value)) = trimmed.split_once(':') else {
                return Err(FrameError::MissingLength);
            };
            if name.trim().eq_ignore_ascii_case("content-length") {
                let parsed = value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| FrameError::BadLength)?;
                length = Some(parsed);
            }
        }
        let length = length.ok_or(FrameError::MissingLength)?;
        if length == 0 {
            return Err(FrameError::BadLength);
        }
        if length > MAX_BODY {
            return Err(FrameError::TooLarge);
        }
        let mut body = vec![0u8; length];
        self.reader.read_exact(&mut body)?;
        String::from_utf8(body)
            .map(Some)
            .map_err(|_| FrameError::BadUtf8)
    }
}

/// Writes framed messages.
pub struct FrameWriter<W: Write> {
    writer: W,
}

impl<W: Write> FrameWriter<W> {
    /// Wraps a writer.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Writes one message and flushes it.
    pub fn write(&mut self, text: &str) -> io::Result<()> {
        write!(self.writer, "Content-Length: {}\r\n\r\n", text.len())?;
        self.writer.write_all(text.as_bytes())?;
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn framed(text: &str) -> Vec<u8> {
        format!("Content-Length: {}\r\n\r\n{}", text.len(), text).into_bytes()
    }

    #[test]
    fn should_read_headers_case_insensitively_and_ignore_extras() {
        let bytes = b"Content-Type: application/vscode-jsonrpc; charset=utf-8\r\ncontent-length: 2\r\n\r\n{}";
        let mut reader = FrameReader::new(Cursor::new(bytes.to_vec()));
        assert_eq!(reader.read().expect("frame").as_deref(), Some("{}"));
    }

    /// A reader that yields one byte at a time, so the test exercises a body
    /// split across many underlying reads.
    struct OneByte<'a> {
        bytes: &'a [u8],
        position: usize,
    }

    impl std::io::Read for OneByte<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            if self.position >= self.bytes.len() {
                return Ok(0);
            }
            buffer[0] = self.bytes[self.position];
            self.position += 1;
            Ok(1)
        }
    }

    impl std::io::BufRead for OneByte<'_> {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            Ok(&self.bytes[self.position..])
        }

        fn consume(&mut self, amount: usize) {
            self.position = (self.position + amount).min(self.bytes.len());
        }
    }

    #[test]
    fn should_read_a_body_split_across_reads() {
        let bytes = framed("{\"id\":1}");
        let source = OneByte {
            bytes: &bytes,
            position: 0,
        };
        let mut reader = FrameReader::new(std::io::BufReader::new(source));
        assert_eq!(reader.read().expect("frame").as_deref(), Some("{\"id\":1}"));
    }

    #[test]
    fn should_reject_a_missing_or_bad_length() {
        let mut missing = FrameReader::new(Cursor::new(b"X: 1\r\n\r\n".to_vec()));
        assert!(matches!(missing.read(), Err(FrameError::MissingLength)));
        let mut bad = FrameReader::new(Cursor::new(b"Content-Length: x\r\n\r\n".to_vec()));
        assert!(matches!(bad.read(), Err(FrameError::BadLength)));
        let mut zero = FrameReader::new(Cursor::new(b"Content-Length: 0\r\n\r\n".to_vec()));
        assert!(matches!(zero.read(), Err(FrameError::BadLength)));
    }

    #[test]
    fn should_report_a_clean_eof_and_a_truncated_header() {
        let mut empty = FrameReader::new(Cursor::new(Vec::new()));
        assert!(matches!(empty.read(), Ok(None)));
        let mut truncated = FrameReader::new(Cursor::new(b"Content-Length: 4\r\n\r\n".to_vec()));
        assert!(matches!(truncated.read(), Err(FrameError::Io(_)) | Err(FrameError::Eof)));
    }

    #[test]
    fn should_round_trip_a_written_message() {
        let mut buffer = Vec::new();
        FrameWriter::new(&mut buffer).write("{\"id\":1}").expect("write");
        assert_eq!(buffer, framed("{\"id\":1}"));
        let mut reader = FrameReader::new(Cursor::new(buffer));
        assert_eq!(reader.read().expect("frame").as_deref(), Some("{\"id\":1}"));
    }
}
