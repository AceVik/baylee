use super::*;
use serde_json::json;
use std::collections::BTreeMap;

pub(super) struct Tensor {
    pub dtype: &'static str,
    pub shape: Vec<usize>,
    pub bytes: Vec<u8>,
}
impl Tensor {
    pub fn new(dtype: &'static str, shape: Vec<usize>, bytes: Vec<u8>) -> Self {
        Self {
            dtype,
            shape,
            bytes,
        }
    }
    pub fn ints(shape: Vec<usize>, values: impl IntoIterator<Item = i64>) -> Self {
        Self::new(
            "i64",
            shape,
            values.into_iter().flat_map(i64::to_le_bytes).collect(),
        )
    }
    pub fn bools(shape: Vec<usize>, values: impl IntoIterator<Item = bool>) -> Self {
        Self::new("u8", shape, values.into_iter().map(u8::from).collect())
    }
    pub fn floats(shape: Vec<usize>, values: impl IntoIterator<Item = f32>) -> Self {
        Self::new(
            "f32",
            shape,
            values.into_iter().flat_map(f32::to_le_bytes).collect(),
        )
    }
    pub fn width(&self) -> usize {
        match self.dtype {
            "i64" => 8,
            "f32" => 4,
            _ => 1,
        }
    }
}

/// One bounded contiguous history segment, padded spatially in Rust only.
pub struct Batch {
    /// Named dtype/shape/byte-range descriptors and full history identity.
    pub descriptor: Value,
    /// Contiguous little-endian array bytes, never ordinary BST4 records.
    pub payload: Vec<u8>,
}

pub(super) fn stack(frames: &[BTreeMap<String, Tensor>], identity: &Value) -> Result<Batch> {
    let first = frames.first().context("empty tensor request")?;
    let mut descriptor = Vec::new();
    let mut payload = Vec::new();
    for (name, tensor) in first {
        let rank = tensor.shape.len();
        let mut shape = tensor.shape.clone();
        for frame in frames {
            let current = frame.get(name).context("inconsistent tensor names")?;
            ensure!(
                frame.len() == first.len()
                    && current.dtype == tensor.dtype
                    && current.shape.len() == rank,
                "inconsistent tensor schema"
            );
            for (size, actual) in shape.iter_mut().zip(&current.shape) {
                *size = (*size).max(*actual);
            }
        }
        let elements = shape
            .iter()
            .try_fold(1_usize, |n, d| n.checked_mul(*d))
            .context("tensor size overflow")?;
        let bytes = elements
            .checked_mul(tensor.width())
            .and_then(|n| n.checked_mul(frames.len()))
            .context("tensor byte overflow")?;
        ensure!(
            bytes <= MAX_BYTES
                && payload
                    .len()
                    .checked_add(bytes)
                    .is_some_and(|n| n <= MAX_BYTES),
            "tensor batch exceeds memory bound"
        );
        let offset = payload.len();
        payload.resize(offset + bytes, 0);
        // Padding for absent entity/seat references must stay -1, never refer to entity zero.
        if matches!(name.as_str(), "cand_entity_idx" | "cand_seat_idx") {
            payload[offset..].fill(255);
        }
        for (time, frame) in frames.iter().enumerate() {
            let current = &frame[name];
            let count = current.shape.iter().product::<usize>();
            ensure!(
                current.bytes.len() == count * current.width(),
                "tensor internal shape mismatch"
            );
            for source in 0..count {
                let mut remainder = source;
                let mut target = 0;
                let mut stride = 1;
                for axis in (0..rank).rev() {
                    target += (remainder % current.shape[axis]) * stride;
                    remainder /= current.shape[axis];
                    stride *= shape[axis];
                }
                let width = tensor.width();
                let dest = offset + (time * elements + target) * width;
                payload[dest..dest + width]
                    .copy_from_slice(&current.bytes[source * width..(source + 1) * width]);
            }
        }
        let mut full = vec![frames.len()];
        full.extend(shape);
        descriptor.push(
            json!({"name":name,"dtype":tensor.dtype,"shape":full,"offset":offset,"nbytes":bytes}),
        );
    }
    Ok(Batch {
        descriptor: json!({"schema":SCHEMA,"identity":identity,"arrays":descriptor,"payload_bytes":payload.len()}),
        payload,
    })
}
