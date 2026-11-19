//! Recorded model inputs: a file of the rows a net was really asked, so an
//! execution provider can be timed and checked against the CPU away from the
//! engine (`hw_capture4` writes it on the trainer's machine, `hw_bench` reads
//! it on the device under test).
//!
//! The file is `ROWDUMP_MAGIC`, a length-prefixed JSON header, then one entry
//! per row: a length-prefixed JSON [`RowMeta`] and its tensors, each a name,
//! an element type, its dimensions and its little-endian elements. A row's
//! tensors carry a leading batch axis of one. The same layout holds a net's
//! outputs (a reference file), with the outputs as the tensors.

use std::io::{self, Read, Write};

use anyhow::{Context as _, bail, ensure};
use serde::{Deserialize, Serialize};

/// The first bytes of a row dump.
pub const ROWDUMP_MAGIC: &[u8; 8] = b"BAYROWS1";

/// Largest header, row metadata or tensor a reader accepts.
const MAX_BLOB: usize = 256 * 1024 * 1024;

/// Which step a row was, as the player that ran it knew it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowMeta {
    /// Game index within the dump.
    pub game: u32,
    /// Seat whose step this is.
    pub seat: u8,
    /// Thinking steps `K` the row was run with; `0` is an observation.
    pub k: u8,
    /// The graph that ran it (`model.k{graph}.onnx`).
    pub graph: u8,
    /// Policy head kind (`HeadKind4` as a number).
    pub head: u8,
    /// Pending kind (`PendingKind4` as a number).
    pub pending_kind: u8,
    /// Real (unpadded) candidates.
    pub candidates: u32,
    /// Real (unpadded) pairs.
    pub pairs: u32,
    /// `PendingKind4/HeadKind4` by name, for reports.
    #[serde(default)]
    pub label: String,
}

/// A tensor's elements.
#[derive(Clone, Debug, PartialEq)]
pub enum TensorData {
    /// `float32`.
    F32(Vec<f32>),
    /// `int64`.
    I64(Vec<i64>),
    /// `bool`.
    Bool(Vec<bool>),
}

impl TensorData {
    /// Number of elements.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::F32(v) => v.len(),
            Self::I64(v) => v.len(),
            Self::Bool(v) => v.len(),
        }
    }

    /// Whether there are no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    const fn tag(&self) -> u8 {
        match self {
            Self::F32(_) => 0,
            Self::I64(_) => 1,
            Self::Bool(_) => 2,
        }
    }
}

/// One named tensor of a row.
#[derive(Clone, Debug, PartialEq)]
pub struct DumpTensor {
    /// Input or output name in the graph.
    pub name: String,
    /// Dimensions, batch axis first.
    pub dims: Vec<usize>,
    /// Elements in row-major order.
    pub data: TensorData,
}

impl DumpTensor {
    /// A tensor, checked to hold as many elements as its dimensions say.
    ///
    /// # Errors
    /// When the element count and the dimensions disagree.
    pub fn new(name: &str, dims: Vec<usize>, data: TensorData) -> anyhow::Result<Self> {
        let expected: usize = dims.iter().product();
        ensure!(
            expected == data.len(),
            "{name}: {} elements for dimensions {dims:?}",
            data.len()
        );
        Ok(Self {
            name: name.to_owned(),
            dims,
            data,
        })
    }

    /// The `float32` elements, if that is the element type.
    #[must_use]
    pub fn f32s(&self) -> Option<&[f32]> {
        match &self.data {
            TensorData::F32(v) => Some(v),
            _ => None,
        }
    }
}

/// One recorded row: its metadata and tensors.
#[derive(Clone, Debug, PartialEq)]
pub struct DumpRow {
    /// What step it was.
    pub meta: RowMeta,
    /// Its tensors.
    pub tensors: Vec<DumpTensor>,
}

impl DumpRow {
    /// The tensor named `name`.
    #[must_use]
    pub fn tensor(&self, name: &str) -> Option<&DumpTensor> {
        self.tensors.iter().find(|t| t.name == name)
    }
}

fn put_blob(out: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    let len = u32::try_from(bytes.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "blob over 4 GiB"))?;
    out.write_all(&len.to_le_bytes())?;
    out.write_all(bytes)
}

fn get_u32(input: &mut impl Read) -> io::Result<u32> {
    let mut b = [0_u8; 4];
    input.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn get_blob(input: &mut impl Read) -> anyhow::Result<Vec<u8>> {
    let len = get_u32(input)? as usize;
    ensure!(len <= MAX_BLOB, "blob of {len} bytes is over the limit");
    let mut bytes = vec![0_u8; len];
    input.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// Writes a row dump.
pub struct RowWriter<W: Write> {
    out: W,
    rows: u64,
}

impl<W: Write> RowWriter<W> {
    /// Starts a dump with `header` (free-form JSON: model, sources, counts).
    ///
    /// # Errors
    /// When writing fails.
    pub fn new(mut out: W, header: &serde_json::Value) -> anyhow::Result<Self> {
        out.write_all(ROWDUMP_MAGIC)?;
        put_blob(&mut out, &serde_json::to_vec(header)?)?;
        Ok(Self { out, rows: 0 })
    }

    /// Appends one row.
    ///
    /// # Errors
    /// When writing fails or a tensor is malformed.
    pub fn write(&mut self, row: &DumpRow) -> anyhow::Result<()> {
        put_blob(&mut self.out, &serde_json::to_vec(&row.meta)?)?;
        let count = u16::try_from(row.tensors.len()).context("over 65535 tensors in a row")?;
        self.out.write_all(&count.to_le_bytes())?;
        for t in &row.tensors {
            let name = t.name.as_bytes();
            let name_len = u16::try_from(name.len()).context("tensor name too long")?;
            self.out.write_all(&name_len.to_le_bytes())?;
            self.out.write_all(name)?;
            self.out.write_all(&[t.data.tag()])?;
            let ndim = u8::try_from(t.dims.len()).context("over 255 dimensions")?;
            self.out.write_all(&[ndim])?;
            for &d in &t.dims {
                self.out.write_all(&(d as u64).to_le_bytes())?;
            }
            match &t.data {
                TensorData::F32(v) => {
                    for x in v {
                        self.out.write_all(&x.to_le_bytes())?;
                    }
                }
                TensorData::I64(v) => {
                    for x in v {
                        self.out.write_all(&x.to_le_bytes())?;
                    }
                }
                TensorData::Bool(v) => {
                    for &x in v {
                        self.out.write_all(&[u8::from(x)])?;
                    }
                }
            }
        }
        self.rows += 1;
        Ok(())
    }

    /// Rows written so far.
    #[must_use]
    pub const fn rows(&self) -> u64 {
        self.rows
    }

    /// Flushes and returns the sink.
    ///
    /// # Errors
    /// When flushing fails.
    pub fn finish(mut self) -> anyhow::Result<W> {
        self.out.flush()?;
        Ok(self.out)
    }
}

/// Reads a row dump.
pub struct RowReader<R: Read> {
    input: R,
    /// The dump's header.
    pub header: serde_json::Value,
}

impl<R: Read> RowReader<R> {
    /// Opens a dump and reads its header.
    ///
    /// # Errors
    /// When the input is not a row dump.
    pub fn new(mut input: R) -> anyhow::Result<Self> {
        let mut magic = [0_u8; 8];
        input.read_exact(&mut magic)?;
        ensure!(&magic == ROWDUMP_MAGIC, "not a row dump");
        let header = serde_json::from_slice(&get_blob(&mut input)?)?;
        Ok(Self { input, header })
    }

    /// The next row, `None` at a clean end of the dump.
    ///
    /// # Errors
    /// When a row is cut off or malformed.
    pub fn next_row(&mut self) -> anyhow::Result<Option<DumpRow>> {
        let mut first = [0_u8; 4];
        match self.input.read(&mut first[..1])? {
            0 => return Ok(None),
            _ => self.input.read_exact(&mut first[1..])?,
        }
        let meta_len = u32::from_le_bytes(first) as usize;
        ensure!(meta_len <= MAX_BLOB, "row metadata over the limit");
        let mut meta = vec![0_u8; meta_len];
        self.input.read_exact(&mut meta)?;
        let meta: RowMeta = serde_json::from_slice(&meta)?;
        let mut count = [0_u8; 2];
        self.input.read_exact(&mut count)?;
        let count = u16::from_le_bytes(count);
        let mut tensors = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            tensors.push(self.tensor()?);
        }
        Ok(Some(DumpRow { meta, tensors }))
    }

    fn tensor(&mut self) -> anyhow::Result<DumpTensor> {
        let mut len = [0_u8; 2];
        self.input.read_exact(&mut len)?;
        let mut name = vec![0_u8; usize::from(u16::from_le_bytes(len))];
        self.input.read_exact(&mut name)?;
        let name = String::from_utf8(name)?;
        let mut tag_ndim = [0_u8; 2];
        self.input.read_exact(&mut tag_ndim)?;
        let mut dims = Vec::with_capacity(usize::from(tag_ndim[1]));
        for _ in 0..tag_ndim[1] {
            let mut d = [0_u8; 8];
            self.input.read_exact(&mut d)?;
            dims.push(usize::try_from(u64::from_le_bytes(d))?);
        }
        let n = dims
            .iter()
            .try_fold(1_usize, |acc, &d| acc.checked_mul(d))
            .context("tensor size overflows")?;
        ensure!(n <= MAX_BLOB, "{name}: {n} elements is over the limit");
        let data = match tag_ndim[0] {
            0 => {
                let mut bytes = vec![0_u8; n * 4];
                self.input.read_exact(&mut bytes)?;
                TensorData::F32(
                    bytes
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| f32::from_le_bytes(*c))
                        .collect(),
                )
            }
            1 => {
                let mut bytes = vec![0_u8; n * 8];
                self.input.read_exact(&mut bytes)?;
                TensorData::I64(
                    bytes
                        .as_chunks::<8>()
                        .0
                        .iter()
                        .map(|c| i64::from_le_bytes(*c))
                        .collect(),
                )
            }
            2 => {
                let mut bytes = vec![0_u8; n];
                self.input.read_exact(&mut bytes)?;
                TensorData::Bool(bytes.into_iter().map(|b| b != 0).collect())
            }
            other => bail!("{name}: unknown element type {other}"),
        };
        DumpTensor::new(&name, dims, data)
    }

    /// Every remaining row.
    ///
    /// # Errors
    /// As [`Self::next_row`].
    pub fn read_all(&mut self) -> anyhow::Result<Vec<DumpRow>> {
        let mut rows = Vec::new();
        while let Some(row) = self.next_row()? {
            rows.push(row);
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_round_trip_and_a_cut_row_is_an_error() {
        let row = DumpRow {
            meta: RowMeta {
                game: 3,
                seat: 1,
                k: 2,
                graph: 2,
                head: 0,
                pending_kind: 7,
                candidates: 2,
                pairs: 0,
                label: "ChooseCards/SubsetPick".to_owned(),
            },
            tensors: vec![
                DumpTensor::new("a", vec![1, 2], TensorData::F32(vec![1.5, -2.0])).unwrap(),
                DumpTensor::new("b", vec![1, 3], TensorData::I64(vec![-1, 0, 7])).unwrap(),
                DumpTensor::new("c", vec![1], TensorData::Bool(vec![true])).unwrap(),
            ],
        };
        let mut w = RowWriter::new(Vec::new(), &serde_json::json!({"model": "m"})).unwrap();
        w.write(&row).unwrap();
        w.write(&row).unwrap();
        let bytes = w.finish().unwrap();
        let mut r = RowReader::new(bytes.as_slice()).unwrap();
        assert_eq!(r.header["model"], "m");
        assert_eq!(r.read_all().unwrap(), vec![row.clone(), row]);
        let cut = &bytes[..bytes.len() - 3];
        let mut r = RowReader::new(cut).unwrap();
        assert!(r.next_row().is_ok());
        assert!(r.next_row().is_err());
        assert!(DumpTensor::new("x", vec![2, 2], TensorData::F32(vec![0.0])).is_err());
    }
}
