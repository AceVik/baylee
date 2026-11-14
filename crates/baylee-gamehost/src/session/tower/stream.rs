//! One bounded reusable line buffer, independent of the total retained record size.
use super::{Digest, Line, Refused, ReplayBudget, Sha256};
use std::io::{BufRead, Read};

pub(super) struct Lines<B> {
    source: B,
    buffer: Vec<u8>,
    hash: Sha256,
    bytes: usize,
    count: usize,
    limits: ReplayBudget,
}

impl<B: BufRead> Lines<B> {
    pub(super) fn new(source: B, limits: ReplayBudget) -> Self {
        Self {
            source,
            buffer: Vec::new(),
            hash: Sha256::new(),
            bytes: 0,
            count: 0,
            limits,
        }
    }

    pub(super) fn next(&mut self) -> Result<Option<Line>, Refused> {
        self.buffer.clear();
        // Read at most one byte beyond each bound to distinguish EOF from excess.
        let allowance = (self.limits.line_bytes + 2).min(self.limits.bytes - self.bytes + 1);
        let read = (&mut self.source)
            .take(allowance as u64)
            .read_until(b'\n', &mut self.buffer)
            .map_err(|_| Refused::Invalid)?;
        if read == 0 {
            return Ok(None);
        }
        if read > self.limits.bytes - self.bytes
            || read > self.limits.line_bytes + 1
            || self.count >= self.limits.lines
        {
            return Err(Refused::Budget);
        }
        if self.buffer.last() != Some(&b'\n') || read == 1 {
            return Err(Refused::Invalid);
        }
        self.hash.update(&self.buffer);
        self.bytes += read;
        self.count += 1;
        serde_json::from_slice(&self.buffer[..read - 1])
            .map(Some)
            .map_err(|_| Refused::Invalid)
    }

    pub(super) fn digest(self) -> [u8; 32] {
        self.hash.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn enormous_unterminated_lines_are_not_read_or_allocated_in_full() {
        let input = vec![b'x'; 1_000_000];
        let mut source = Cursor::new(input);
        let limit = ReplayBudget {
            line_bytes: 32,
            ..ReplayBudget::default()
        };
        let mut lines = Lines::new(&mut source, limit);
        assert_eq!(lines.next(), Err(Refused::Budget));
        assert!(lines.buffer.len() <= 34);
        drop(lines);
        assert_eq!(source.position(), 34);
    }
}
