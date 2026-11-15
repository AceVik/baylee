//! A sequence played frame after frame: the file's bytes, its tables, and a
//! decoder for each track, starting over at the first frame (a key frame)
//! when the sequence ends.

use std::sync::Arc;

use crate::dav1d::Decoder;
use crate::{AlignedPlanes, Bounds, Error, Frame, Sequence};

enum Source {
    Static(&'static [u8]),
    Shared(Arc<[u8]>),
}

impl Source {
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Static(bytes) => bytes,
            Self::Shared(bytes) => bytes,
        }
    }
}

/// Plays one sequence. Owns everything it reads, so it can live on a worker
/// thread while the caller keeps the bytes for a still elsewhere.
pub struct Player {
    bytes: Source,
    sequence: Sequence,
    colour: Decoder,
    alpha: Option<Decoder>,
    /// The frame the next call decodes.
    next: usize,
    /// The alpha track's last frame, whose luma is copied into the colour
    /// frame's alpha plane.
    alpha_frame: Frame,
}

impl Player {
    /// Reads the sequence in `bytes` within `bounds` and opens a decoder for
    /// each of its tracks on `threads` threads.
    ///
    /// # Errors
    ///
    /// The file is refused by [`Sequence::read`] or a decoder will not open.
    pub fn open(bytes: Arc<[u8]>, bounds: &Bounds, threads: u32) -> Result<Self, Error> {
        Self::open_source(Source::Shared(bytes), bounds, threads)
    }

    /// Opens a sequence backed by `'static` bytes (`include_bytes!`) without
    /// cloning the file onto the heap.
    ///
    /// # Errors
    ///
    /// The file is refused by [`Sequence::read`] or a decoder will not open.
    pub fn open_static(bytes: &'static [u8], bounds: &Bounds, threads: u32) -> Result<Self, Error> {
        Self::open_source(Source::Static(bytes), bounds, threads)
    }

    fn open_source(bytes: Source, bounds: &Bounds, threads: u32) -> Result<Self, Error> {
        let sequence = Sequence::read(bytes.as_slice(), bounds)?;
        let colour = Decoder::open(threads, bounds.max_pixels())?;
        let alpha = match sequence.alpha {
            Some(_) => Some(Decoder::open(1, bounds.max_pixels())?),
            None => None,
        };
        Ok(Self {
            bytes,
            sequence,
            colour,
            alpha,
            next: 0,
            alpha_frame: Frame::default(),
        })
    }

    /// The tables this player reads.
    #[must_use]
    pub fn sequence(&self) -> &Sequence {
        &self.sequence
    }

    /// How many frames one pass has.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.sequence.colour.samples.len()
    }

    /// The frame the next [`Self::advance`] decodes.
    #[must_use]
    pub fn next_index(&self) -> usize {
        self.next
    }

    /// Decodes the next frame into `frame` (its buffers reused) and returns
    /// its index and how long it is shown in whole milliseconds. After the
    /// last frame the decoder forgets its references and the next call decodes
    /// the first again, so a caller that holds a sequence which does not
    /// repeat stops calling.
    ///
    /// # Errors
    ///
    /// The decoder refused a sample or made a frame its track did not
    /// declare. The player is then left at the frame that failed.
    pub fn advance(&mut self, frame: &mut Frame) -> Result<(usize, u32), Error> {
        let index = self.next;
        let raw = self.bytes.as_slice();
        let track = &self.sequence.colour;
        let sample = track.samples[index];
        let bytes = &raw[sample.offset..sample.offset + sample.len];
        self.colour
            .decode(bytes, track.width, track.height, frame, false)?;
        match (&mut self.alpha, &self.sequence.alpha) {
            (Some(decoder), Some(alpha)) => {
                let a = alpha.samples[index];
                let bytes = &raw[a.offset..a.offset + a.len];
                decoder.decode(
                    bytes,
                    alpha.width,
                    alpha.height,
                    &mut self.alpha_frame,
                    true,
                )?;
                let plane = frame.alpha.get_or_insert_with(Vec::new);
                plane.clear();
                plane.extend_from_slice(&self.alpha_frame.y);
            }
            _ => frame.alpha = None,
        }
        let ms = sample.duration_ms(track.timescale).min(u64::from(u32::MAX)) as u32;
        self.step_index();
        Ok((index, ms))
    }

    /// Decodes the next 8-bit 4:2:0 colour frame directly into 256-byte
    /// row-aligned `planes` (zero heap allocations after first call) and
    /// returns the decoded sample index within the pass.
    ///
    /// # Errors
    ///
    /// The decoder refused a sample or made a frame its track did not declare.
    pub fn advance_aligned(&mut self, planes: &mut AlignedPlanes) -> Result<usize, Error> {
        let index = self.next;
        let raw = self.bytes.as_slice();
        let track = &self.sequence.colour;
        let sample = track.samples[index];
        let bytes = &raw[sample.offset..sample.offset + sample.len];
        self.colour
            .decode_aligned(bytes, track.width, track.height, planes)?;
        self.step_index();
        Ok(index)
    }

    fn step_index(&mut self) {
        self.next += 1;
        if self.next == self.sequence.colour.samples.len() {
            self.next = 0;
            self.colour.flush();
            if let Some(decoder) = &mut self.alpha {
                decoder.flush();
            }
        }
    }
}
