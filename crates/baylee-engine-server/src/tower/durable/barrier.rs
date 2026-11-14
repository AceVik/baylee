use baylee_protocol::v1::Envelope;
use baylee_tower::{
    Refusal,
    v2::execution::{MAX_BATCH_CHUNKS, MAX_CHUNK_BYTES, Prefix},
};
use prost::Message as _;

const MAX_VIEW_BYTES: usize = 16 * 1024 * 1024;
const MAX_VIEW_FRAMES: usize = 4096;

/// One complete operation. Every chunk must commit in one authority transaction.
pub struct Batch<'a> {
    /// Assignment generation; checked again by the authenticated authority.
    pub fence: u64,
    /// Last acknowledged prefix, never a speculative later head.
    pub expected: Prefix,
    pieces: &'a [Vec<u8>],
}

impl Batch<'_> {
    /// Canonical ordered chunks, each at most 256 KiB. Never acknowledge a subset.
    pub fn chunks(&self) -> impl Iterator<Item = &[u8]> {
        self.pieces
            .iter()
            .flat_map(|piece| piece.chunks(MAX_CHUNK_BYTES))
    }
}

pub(super) struct Barrier {
    package: [u8; 32],
    fence: u64,
    limit: u64,
    acknowledged: Prefix,
    expected_ack: Option<Prefix>,
    pieces: Vec<Vec<u8>>,
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
            pieces: Vec::new(),
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
        let mut count = 0;
        for piece in &pieces {
            for chunk in piece.chunks(MAX_CHUNK_BYTES) {
                count += 1;
                if count > MAX_BATCH_CHUNKS {
                    return Err(Refusal::Capacity);
                }
                head = head.extended(self.package, self.limit, chunk)?;
            }
        }
        if pieces.is_empty() {
            return Ok(frames);
        }
        if pieces.last().and_then(|piece| piece.last()) != Some(&b'\n') {
            return Err(Refusal::Invalid);
        }
        self.pieces = pieces;
        self.frames = frames;
        self.expected_ack = Some(head);
        Ok(Vec::new())
    }

    pub(super) fn waiting(&self) -> bool {
        self.expected_ack.is_some()
    }
    pub(super) fn prefix(&self) -> Prefix {
        self.acknowledged
    }

    pub(super) fn batch(&self) -> Option<Batch<'_>> {
        self.expected_ack?;
        Some(Batch {
            fence: self.fence,
            expected: self.acknowledged,
            pieces: &self.pieces,
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
        self.pieces.clear();
        self.expected_ack = None;
        Ok(std::mem::take(&mut self.frames))
    }
}
