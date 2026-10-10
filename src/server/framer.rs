//! Line framing for the TCP server.

use std::io;

use tokio::io::{AsyncRead, AsyncReadExt};

/// Splits a stream into lines and returns each one as a slice of its own buffer.
///
/// [`next`](Self::next) needs Polonius. See "Nightly features > Polonius" in the README.
#[derive(Debug)]
pub struct LineFramer {
    buf: Vec<u8>,
    /// Start of unread data.
    start: usize,
    /// End of unread data.
    end: usize,
    /// Searched for a newline up to here.
    scanned: usize,
    limit: usize,
}

impl LineFramer {
    /// A framer that rejects lines longer than `limit` bytes.
    pub fn new(limit: usize) -> Self {
        LineFramer {
            buf: vec![0; limit.clamp(1, 8 * 1024)],
            start: 0,
            end: 0,
            scanned: 0,
            limit,
        }
    }

    /// Returns the next line without its `\n`, or `None` at end of stream.
    pub async fn next<R: AsyncRead + Unpin>(
        &mut self,
        reader: &mut R,
    ) -> io::Result<Option<&[u8]>> {
        loop {
            if let Some(line) = self.buffered_line() {
                return Ok(Some(line));
            }
            // Stable rustc stops here: it thinks `line` still borrows `self`.
            self.compact();
            if self.end == self.buf.len() {
                if self.end > self.limit {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "line exceeds limit",
                    ));
                }
                // +1 leaves room for the newline.
                let grown = (self.buf.len() * 2).min(self.limit + 1);
                self.buf.resize(grown, 0);
            }
            let read = reader.read(&mut self.buf[self.end..]).await?;
            if read == 0 {
                let rest = self.end;
                (self.start, self.end, self.scanned) = (0, 0, 0);
                return Ok((rest > 0).then(|| &self.buf[..rest]));
            }
            self.end += read;
        }
    }

    fn buffered_line(&mut self) -> Option<&[u8]> {
        let newline = self.buf[self.scanned..self.end]
            .iter()
            .position(|&b| b == b'\n');
        let Some(offset) = newline else {
            self.scanned = self.end;
            return None;
        };
        let line_end = self.scanned + offset;
        let line = &self.buf[self.start..line_end];
        self.start = line_end + 1;
        self.scanned = self.start;
        Some(line)
    }

    /// Moves the unread tail to the front so the buffer can be refilled.
    fn compact(&mut self) {
        if self.start > 0 {
            self.buf.copy_within(self.start..self.end, 0);
            self.end -= self.start;
            self.scanned -= self.start;
            self.start = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn frames_lines_across_reads() {
        // A tiny limit forces partial reads and buffer growth.
        let mut framer = LineFramer::new(8);
        let mut input: &[u8] = b"ab\ncdefgh\n\nxyz";
        let mut lines = Vec::new();
        while let Some(line) = framer.next(&mut input).await.unwrap() {
            lines.push(line.to_vec());
        }
        assert_eq!(lines, [&b"ab"[..], b"cdefgh", b"", b"xyz"]);
    }

    #[tokio::test]
    async fn rejects_overlong_lines() {
        let mut framer = LineFramer::new(4);
        let mut input: &[u8] = b"12345678\n";
        let err = framer.next(&mut input).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
