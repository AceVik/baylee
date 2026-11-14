use super::{Control, Prefix};
use crate::host::{Acknowledged, Batch, Lease};
use baylee_tower::{
    Refusal,
    v2::execution::{Execution, MAX_BATCH_BYTES},
};

/// One host's bounded multipart operation; no partial ACK or interleaved batch.
pub struct Assembly {
    lease: Lease,
    package: [u8; 32],
    limit: u64,
    prefix: Prefix,
    staged: Prefix,
    count: u8,
    chunks: Vec<Vec<u8>>,
    waiting: bool,
    refused: bool,
}

impl Assembly {
    /// Bind to the authority's exact claim and current durable head.
    /// # Errors
    /// Invalid budget, prefix or zero generation.
    pub fn new(
        lease: Lease,
        package: [u8; 32],
        limit: u64,
        prefix: Prefix,
    ) -> Result<Self, Refusal> {
        let mut execution = Execution::new(package, limit)?;
        execution.fence = lease.fence;
        execution.prefix = prefix;
        execution.validate()?;
        if lease.fence == 0 {
            return Err(Refusal::Invalid);
        }
        Ok(Self {
            lease,
            package,
            limit,
            prefix,
            staged: prefix,
            count: 0,
            chunks: Vec::new(),
            waiting: false,
            refused: false,
        })
    }

    /// Buffer an ordered part; only the final part returns one whole HTTP batch.
    /// Any invalid sequence poisons this assembler; restart from authority state.
    /// # Errors
    /// Wrong generation/head/order, interleaving, bounds or incomplete final line.
    pub fn push(&mut self, control: &Control, data: Vec<u8>) -> Result<Option<Batch>, Refusal> {
        let result = self.part(control, data);
        if result.is_err() {
            self.refused = true;
        }
        result
    }

    fn part(&mut self, control: &Control, data: Vec<u8>) -> Result<Option<Batch>, Refusal> {
        if self.refused || self.waiting {
            return Err(Refusal::Stale);
        }
        control.validate(&data)?;
        let Control::Append {
            fence,
            expected,
            chunks,
            index,
        } = control
        else {
            return Err(Refusal::Invalid);
        };
        if *fence != self.lease.fence
            || *expected != self.prefix
            || usize::from(*index) != self.chunks.len()
            || (*index != 0 && self.count != *chunks)
        {
            return Err(Refusal::Stale);
        }
        let next = self.staged.extended(self.package, self.limit, &data)?;
        if next.bytes - self.prefix.bytes > MAX_BATCH_BYTES as u64 {
            return Err(Refusal::Capacity);
        }
        let last = *index + 1 == *chunks;
        if last && data.last() != Some(&b'\n') {
            return Err(Refusal::Invalid);
        }
        self.count = *chunks;
        self.staged = next;
        self.chunks.push(data);
        if !last {
            return Ok(None);
        }
        self.waiting = true;
        Ok(Some(Batch {
            lease: self.lease,
            expected: self.prefix,
            chunks: std::mem::take(&mut self.chunks),
        }))
    }

    /// Authorize the next operation only after the authority ACK matches all bytes.
    /// # Errors
    /// No complete pending batch or a partial/fabricated/wrong-generation ACK.
    pub fn acknowledge(&mut self, ack: Acknowledged) -> Result<(), Refusal> {
        if self.refused
            || !self.waiting
            || ack.fence != self.lease.fence
            || ack.prefix != self.staged
        {
            self.refused = true;
            return Err(Refusal::Stale);
        }
        self.prefix = self.staged;
        self.waiting = false;
        self.count = 0;
        Ok(())
    }

    /// True only outside an uncommitted or invalid operation, at this exact head.
    #[must_use]
    pub fn can_seal(&self, expected: Prefix) -> bool {
        !self.refused
            && !self.waiting
            && self.chunks.is_empty()
            && expected == self.prefix
            && expected.chunks != 0
    }
}
