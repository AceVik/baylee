//! Reconstruct the exact acknowledged prefix using at most one raw chunk buffer.
use super::{Binding, Refused};
use baylee_tower::v2::execution::{Execution, MAX_CHUNK_BYTES, Prefix};
use std::io::{self, BufRead, Read};

pub(in crate::tower) struct Committed<R> {
    source: R,
    execution: Execution,
    expected: Prefix,
    buffer: Vec<u8>,
    offset: usize,
    eof: bool,
}

impl<R: Read> Committed<R> {
    pub(in crate::tower) fn new(source: R, binding: Binding) -> Result<Self, Refused> {
        if binding.fence == 0 || binding.prefix.chunks == 0 {
            return Err(Refused::Invalid);
        }
        let execution = Execution::new(binding.package, binding.prefix.bytes)
            .and_then(|state| state.claimed(0, 1))
            .map_err(|_| Refused::Invalid)?;
        let mut check = execution.clone();
        check.prefix = binding.prefix;
        check.validate().map_err(|_| Refused::Invalid)?;
        Ok(Self {
            source,
            execution,
            expected: binding.prefix,
            buffer: Vec::new(),
            offset: 0,
            eof: false,
        })
    }

    fn advance(&mut self) -> io::Result<()> {
        self.buffer.clear();
        self.offset = 0;
        if self.eof {
            return Ok(());
        }
        if self.execution.prefix.chunks == self.expected.chunks {
            let mut excess = [0];
            if self.execution.prefix != self.expected || self.source.read(&mut excess)? != 0 {
                return Err(invalid());
            }
            self.eof = true;
            return Ok(());
        }
        let mut length = [0; 4];
        self.source.read_exact(&mut length)?;
        let length = baylee_worker::frame::length(length, MAX_CHUNK_BYTES)?;
        self.buffer.resize(length, 0);
        self.source.read_exact(&mut self.buffer)?;
        self.execution = self
            .execution
            .appended(1, 0, self.execution.prefix, &self.buffer)
            .map_err(|_| invalid())?;
        if self.execution.prefix.chunks == self.expected.chunks
            && self.execution.prefix != self.expected
        {
            return Err(invalid());
        }
        Ok(())
    }
}

impl<R: Read> Read for Committed<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        let bytes = self.fill_buf()?;
        let take = output.len().min(bytes.len());
        output[..take].copy_from_slice(&bytes[..take]);
        self.consume(take);
        Ok(take)
    }
}

impl<R: Read> BufRead for Committed<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.offset == self.buffer.len() {
            self.advance()?;
        }
        Ok(&self.buffer[self.offset..])
    }
    fn consume(&mut self, amount: usize) {
        self.offset += amount.min(self.buffer.len() - self.offset);
    }
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid Tower record commitment",
    )
}
