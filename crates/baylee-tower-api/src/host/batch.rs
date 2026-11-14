//! Bounded binary framing. Chunk boundaries survive retries without JSON byte expansion.
use super::{Lease, Prefix};
use baylee_tower::{
    Refusal,
    v2::execution::{MAX_BATCH_BYTES, MAX_BATCH_CHUNKS, MAX_CHUNK_BYTES},
};
use serde::{Deserialize, Serialize};

const MAX_METADATA: usize = 4096;
/// Entire HTTP body ceiling, including lengths and bounded metadata.
pub const MAX_FRAME_BYTES: usize = MAX_BATCH_BYTES + MAX_METADATA + 4 * (MAX_BATCH_CHUNKS + 1);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    lease: Lease,
    expected: Prefix,
    chunks: u8,
}

/// One complete operation, never printable. Store validation precedes any durable ACK.
pub struct Batch {
    /// Exact assigned host generation.
    pub lease: Lease,
    /// Previous acknowledged record head.
    pub expected: Prefix,
    /// At most 64 nonempty chunks, total at most eight MiB, ending at a record boundary.
    pub chunks: Vec<Vec<u8>>,
}

impl Batch {
    /// Encode length-prefixed metadata followed by the unchanged length-prefixed chunks.
    /// # Errors
    /// Invalid chunk/count/total bounds or incomplete final record line.
    pub fn encode(&self) -> Result<Vec<u8>, Refusal> {
        let total = self.validate()?;
        let metadata = serde_json::to_vec(&Metadata {
            lease: self.lease,
            expected: self.expected,
            chunks: u8::try_from(self.chunks.len()).map_err(|_| Refusal::Capacity)?,
        })
        .map_err(|_| Refusal::Invalid)?;
        if metadata.len() > MAX_METADATA {
            return Err(Refusal::Capacity);
        }
        let mut bytes = Vec::with_capacity(total + metadata.len() + 4 * (self.chunks.len() + 1));
        push(&mut bytes, &metadata)?;
        for chunk in &self.chunks {
            push(&mut bytes, chunk)?;
        }
        Ok(bytes)
    }

    /// Decode only the fixed bounded frame; trailing or truncated bytes are rejected.
    /// # Errors
    /// Oversized, malformed, ambiguous or incomplete whole operation.
    pub fn decode(mut bytes: &[u8]) -> Result<Self, Refusal> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(Refusal::Capacity);
        }
        let metadata: Metadata = serde_json::from_slice(take(&mut bytes, MAX_METADATA)?)
            .map_err(|_| Refusal::Invalid)?;
        if !(1..=MAX_BATCH_CHUNKS).contains(&usize::from(metadata.chunks)) {
            return Err(Refusal::Capacity);
        }
        let mut chunks = Vec::with_capacity(usize::from(metadata.chunks));
        let mut total: usize = 0;
        for _ in 0..metadata.chunks {
            let chunk = take(&mut bytes, MAX_CHUNK_BYTES)?;
            total = total.checked_add(chunk.len()).ok_or(Refusal::Capacity)?;
            if total > MAX_BATCH_BYTES {
                return Err(Refusal::Capacity);
            }
            chunks.push(chunk.to_vec());
        }
        if !bytes.is_empty() {
            return Err(Refusal::Invalid);
        }
        let result = Self {
            lease: metadata.lease,
            expected: metadata.expected,
            chunks,
        };
        result.validate()?;
        Ok(result)
    }

    fn validate(&self) -> Result<usize, Refusal> {
        if !(1..=MAX_BATCH_CHUNKS).contains(&self.chunks.len()) {
            return Err(Refusal::Capacity);
        }
        let mut total: usize = 0;
        for chunk in &self.chunks {
            if chunk.is_empty() || chunk.len() > MAX_CHUNK_BYTES {
                return Err(Refusal::Capacity);
            }
            total = total.checked_add(chunk.len()).ok_or(Refusal::Capacity)?;
        }
        if total > MAX_BATCH_BYTES {
            return Err(Refusal::Capacity);
        }
        if self.chunks.last().and_then(|chunk| chunk.last()) != Some(&b'\n') {
            return Err(Refusal::Invalid);
        }
        Ok(total)
    }
}

fn push(bytes: &mut Vec<u8>, part: &[u8]) -> Result<(), Refusal> {
    bytes.extend_from_slice(
        &u32::try_from(part.len())
            .map_err(|_| Refusal::Capacity)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(part);
    Ok(())
}

fn take<'a>(bytes: &mut &'a [u8], max: usize) -> Result<&'a [u8], Refusal> {
    let (length, rest) = bytes.split_at_checked(4).ok_or(Refusal::Invalid)?;
    let length = u32::from_be_bytes(length.try_into().map_err(|_| Refusal::Invalid)?) as usize;
    if length == 0 || length > max {
        return Err(Refusal::Capacity);
    }
    let (part, rest) = rest.split_at_checked(length).ok_or(Refusal::Invalid)?;
    *bytes = rest;
    Ok(part)
}

#[cfg(test)]
mod tests;
