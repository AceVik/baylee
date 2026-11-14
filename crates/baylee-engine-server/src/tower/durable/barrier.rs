use baylee_protocol::v1::Envelope;
use baylee_tower::{
    Refusal,
    v2::execution::{MAX_CHUNK_BYTES, Prefix},
};
use prost::Message as _;
use std::collections::VecDeque;

const MAX_VIEW_BYTES: usize = 16 * 1024 * 1024;
const MAX_VIEW_FRAMES: usize = 4096;

/// One private append. Retries repeat these exact bytes and expected head.
pub struct Append<'a> {
    /// Assignment generation; checked again by the authenticated authority.
    pub fence: u64,
    /// Last acknowledged prefix, never a speculative later head.
    pub expected: Prefix,
    /// At most 256 KiB of canonical record data, no gzip/encoding ambiguity.
    pub data: &'a [u8],
}

pub(super) struct Barrier {
    package: [u8; 32],
    fence: u64,
    limit: u64,
    acknowledged: Prefix,
    expected_ack: Option<Prefix>,
    pieces: VecDeque<Vec<u8>>,
    offset: usize,
    frames: Vec<Envelope>,
}

impl Barrier {
    pub(super) fn new(package: [u8; 32], fence: u64, limit: u64) -> Result<Self, Refusal> {
        baylee_tower::v2::execution::Execution::new(package, limit)?;
        if fence == 0 {
            return Err(Refusal::Invalid);
        }
        Ok(Self {
            package,
            fence,
            limit,
            acknowledged: Prefix::default(),
            expected_ack: None,
            pieces: VecDeque::new(),
            offset: 0,
            frames: Vec::new(),
        })
    }

    pub(super) fn stage(
        &mut self,
        pieces: Vec<Vec<u8>>,
        frames: Vec<Envelope>,
    ) -> Result<Vec<Envelope>, Refusal> {
        if self.waiting() {
            return Err(Refusal::Unavailable);
        }
        if frames.len() > MAX_VIEW_FRAMES {
            return Err(Refusal::Capacity);
        }
        let mut bytes = 0_usize;
        for frame in &frames {
            bytes = bytes
                .checked_add(frame.encoded_len())
                .filter(|bytes| *bytes <= MAX_VIEW_BYTES)
                .ok_or(Refusal::Capacity)?;
        }
        // Validate the whole proposed batch before offering any of it to storage.
        let mut head = self.acknowledged;
        for piece in &pieces {
            for chunk in piece.chunks(MAX_CHUNK_BYTES) {
                head = head.extended(self.package, self.limit, chunk)?;
            }
        }
        if pieces.is_empty() {
            return Ok(frames);
        }
        self.pieces = pieces.into();
        self.frames = frames;
        self.prepare()?;
        Ok(Vec::new())
    }

    pub(super) fn waiting(&self) -> bool {
        self.expected_ack.is_some()
    }
    pub(super) fn prefix(&self) -> Prefix {
        self.acknowledged
    }

    pub(super) fn append(&self) -> Option<Append<'_>> {
        let piece = self.pieces.front()?;
        let end = (self.offset + MAX_CHUNK_BYTES).min(piece.len());
        Some(Append {
            fence: self.fence,
            expected: self.acknowledged,
            data: &piece[self.offset..end],
        })
    }

    pub(super) fn ack(&mut self, fence: u64, prefix: Prefix) -> Result<Vec<Envelope>, Refusal> {
        if fence != self.fence {
            return Err(Refusal::Stale);
        }
        // An exact already processed ACK is harmless, and never releases twice.
        if prefix == self.acknowledged && prefix.chunks != 0 {
            return Ok(Vec::new());
        }
        if self.expected_ack != Some(prefix) {
            return Err(Refusal::Stale);
        }
        self.acknowledged = prefix;
        self.offset += self.append().ok_or(Refusal::Invalid)?.data.len();
        if self
            .pieces
            .front()
            .is_some_and(|piece| self.offset == piece.len())
        {
            self.pieces.pop_front();
            self.offset = 0;
        }
        self.prepare()?;
        if self.waiting() {
            Ok(Vec::new())
        } else {
            Ok(std::mem::take(&mut self.frames))
        }
    }

    fn prepare(&mut self) -> Result<(), Refusal> {
        self.expected_ack = self
            .append()
            .map(|append| {
                self.acknowledged
                    .extended(self.package, self.limit, append.data)
            })
            .transpose()?;
        Ok(())
    }
}
