//! Recording every graph call a player makes (`hw_capture4`): the rows go to a
//! [`baylee_driver::rowdump`] file, so an execution provider can be timed and
//! checked against the CPU on the very inputs the net was asked.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use baylee_driver::rowdump::{DumpRow, RowMeta, RowWriter};

use super::NetPlayer4;
use super::inference::StepRow;
use crate::features4::EncodedCandidates4;

/// Where a player's graph calls are written.
pub struct RowCapture4 {
    writer: RowWriter<BufWriter<File>>,
    game: u32,
    seat: u8,
    decisions: u64,
    observations: u64,
}

impl RowCapture4 {
    /// Starts a dump at `path` with `header`.
    ///
    /// # Errors
    /// When the file cannot be created or written.
    pub fn create(path: &Path, header: &serde_json::Value) -> anyhow::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            writer: RowWriter::new(BufWriter::new(file), header)?,
            game: 0,
            seat: 0,
            decisions: 0,
            observations: 0,
        })
    }

    /// The game and seat the next rows belong to.
    pub const fn set_context(&mut self, game: u32, seat: u8) {
        self.game = game;
        self.seat = seat;
    }

    /// Decision rows (`K > 0`) written so far.
    pub const fn decisions(&self) -> u64 {
        self.decisions
    }

    /// Observation rows (`K = 0`) written so far.
    pub const fn observations(&self) -> u64 {
        self.observations
    }

    pub(super) fn record(
        &mut self,
        row: &StepRow,
        cands: &EncodedCandidates4,
        graph: u8,
        d_model: usize,
    ) -> anyhow::Result<()> {
        let meta = RowMeta {
            game: self.game,
            seat: self.seat,
            k: row.k_val,
            graph,
            head: cands.head as u8,
            pending_kind: cands.pending_kind as u8,
            candidates: u32::try_from(cands.candidates.len())?,
            pairs: u32::try_from(cands.pairs.len())?,
            label: format!("{:?}/{:?}", cands.pending_kind, cands.head),
        };
        if row.k_val == 0 {
            self.observations += 1;
        } else {
            self.decisions += 1;
        }
        self.writer.write(&DumpRow {
            meta,
            tensors: row.dump_tensors(d_model)?,
        })
    }

    /// Flushes the dump.
    ///
    /// # Errors
    /// When flushing fails.
    pub fn finish(self) -> anyhow::Result<()> {
        self.writer.finish()?;
        Ok(())
    }
}

impl NetPlayer4 {
    /// Records every graph call from now on into `capture`; `None` stops it
    /// and hands the capture back.
    pub fn set_capture(&mut self, capture: Option<RowCapture4>) -> Option<RowCapture4> {
        std::mem::replace(&mut self.capture, capture)
    }

    /// The capture, to set its game and seat.
    pub const fn capture_mut(&mut self) -> Option<&mut RowCapture4> {
        self.capture.as_mut()
    }
}
