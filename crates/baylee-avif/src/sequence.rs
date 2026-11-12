//! An AVIF image sequence's tracks and where each frame's bytes lie
//! (ISO/IEC 23000-22 and 14496-12 §8): the colour track, an alpha track
//! that refers to it (`auxl`), and for each, every sample's offset, length
//! and duration. Nothing here decodes; [`crate::Player`] does.

use crate::boxes::{Atom, Cursor, children, full, needed, only};
use crate::{Bounds, Error};

/// The shortest a frame is shown, in milliseconds. A file may state zero;
/// a player that took it at its word would decode as fast as it could.
pub const MIN_FRAME_MS: u32 = 10;

/// One frame's bytes in the file, and how long it is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    /// Where its bytes start in the file.
    pub offset: usize,
    /// How many bytes it has.
    pub len: usize,
    /// How long it is shown, at least [`MIN_FRAME_MS`].
    pub duration_ms: u32,
}

/// One track: its frames' size and its samples.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    /// The track's id, which an alpha track names in its `auxl` reference.
    pub id: u32,
    /// Each frame's width in pixels, as its sample entry states it.
    pub width: u32,
    /// Each frame's height in pixels.
    pub height: u32,
    /// Every frame, in decoding order; the first is a key frame.
    pub samples: Vec<Sample>,
}

impl Track {
    /// One pass through the track, in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        self.samples.iter().map(|s| u64::from(s.duration_ms)).sum()
    }
}

/// A sequence ready to decode: its colour track, the alpha track that goes
/// with it if the picture is transparent, and whether it repeats.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sequence {
    /// The picture's colour.
    pub colour: Track,
    /// Its transparency, frame for frame, when it has any.
    pub alpha: Option<Track>,
    /// Whether the sequence plays again from its start when it ends (the edit
    /// list's repeat flag, which ffmpeg's `-loop 0` sets); otherwise its last
    /// frame stands.
    pub repeats: bool,
}

impl Sequence {
    /// Reads the sequence in `bytes`, refusing anything [`Bounds`] does not
    /// allow before it reserves memory for it.
    ///
    /// # Errors
    ///
    /// The file is too large, cut short, not an `avis` sequence of one or two
    /// AV1 tracks, or states a size, count or duration outside `bounds`.
    pub fn read(bytes: &[u8], bounds: &Bounds) -> Result<Self, Error> {
        if bytes.len() > bounds.max_bytes {
            return Err(Error::TooLarge(bytes.len()));
        }
        let top = children(bytes)?;
        let ftyp = needed(&top, b"ftyp", "no file type")?;
        if !is_sequence(ftyp.body)? {
            return Err(Error::Unsupported("not an image sequence (no avis brand)"));
        }
        let moov = needed(&top, b"moov", "no movie box")?;
        let traks: Vec<Atom<'_>> = children(moov.body)?
            .into_iter()
            .filter(|a| &a.kind == b"trak")
            .collect();
        if traks.is_empty() || traks.len() > 2 {
            return Err(Error::Unsupported("one or two tracks expected"));
        }
        let mut read = Vec::with_capacity(traks.len());
        for trak in &traks {
            read.push(track(trak.body, bytes.len(), bounds)?);
        }
        let colour_at = read
            .iter()
            .position(|t| t.role == Role::Colour)
            .ok_or(Error::Unsupported("no colour track"))?;
        let colour = read.remove(colour_at);
        let alpha = read.pop().and_then(|t| {
            // An alpha track is kept only when it is exactly the colour
            // track's partner: frame for frame, size for size.
            let partner = t.role == Role::Alpha
                && t.aux_for == Some(colour.track.id)
                && t.track.width == colour.track.width
                && t.track.height == colour.track.height
                && t.track.samples.len() == colour.track.samples.len();
            partner.then_some(t.track)
        });
        Ok(Self {
            repeats: colour.repeats,
            colour: colour.track,
            alpha,
        })
    }
}

fn is_sequence(ftyp: &[u8]) -> Result<bool, Error> {
    let mut at = Cursor::new(ftyp);
    let major = at.take(4, "major brand")?;
    at.skip(4, "minor version")?;
    if major == b"avis" {
        return Ok(true);
    }
    Ok(at
        .rest()
        .as_chunks::<4>()
        .0
        .iter()
        .any(|brand| brand == b"avis"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Colour,
    Alpha,
}

struct Read {
    track: Track,
    role: Role,
    aux_for: Option<u32>,
    repeats: bool,
}

fn track(trak: &[u8], file_len: usize, bounds: &Bounds) -> Result<Read, Error> {
    let atoms = children(trak)?;
    let id = track_id(needed(&atoms, b"tkhd", "no track header")?.body)?;
    let aux_for = match only(&atoms, b"tref")? {
        Some(tref) => auxiliary_of(tref.body)?,
        None => None,
    };
    let repeats = match only(&atoms, b"edts")? {
        Some(edts) => match only(&children(edts.body)?, b"elst")? {
            Some(elst) => full(elst.body)?.1 & 1 == 1,
            None => false,
        },
        None => false,
    };
    let mdia = children(needed(&atoms, b"mdia", "no media box")?.body)?;
    let timescale = timescale(needed(&mdia, b"mdhd", "no media header")?.body)?;
    let role = match handler(needed(&mdia, b"hdlr", "no handler")?.body)? {
        h if &h == b"pict" || &h == b"vide" => Role::Colour,
        h if &h == b"auxv" => Role::Alpha,
        _ => {
            return Err(Error::Unsupported(
                "a track that is neither picture nor alpha",
            ));
        }
    };
    let minf = children(needed(&mdia, b"minf", "no media information")?.body)?;
    let stbl = children(needed(&minf, b"stbl", "no sample table")?.body)?;
    let (width, height) = sample_entry(needed(&stbl, b"stsd", "no sample description")?.body)?;
    if width == 0 || height == 0 || width > bounds.max_side || height > bounds.max_side {
        return Err(Error::OutOfBounds("frame size"));
    }
    if bounds
        .exact_side
        .is_some_and(|side| width != side || height != side)
    {
        return Err(Error::OutOfBounds("frame size is not the one allowed"));
    }
    let sizes = sample_sizes(needed(&stbl, b"stsz", "no sample sizes")?.body, bounds)?;
    let count = sizes.len();
    let durations = durations(
        needed(&stbl, b"stts", "no sample times")?.body,
        count,
        timescale,
    )?;
    let total: u64 = durations.iter().map(|&d| u64::from(d)).sum();
    if total > u64::from(bounds.max_duration_ms) {
        return Err(Error::OutOfBounds("duration"));
    }
    let chunks = match (only(&stbl, b"stco")?, only(&stbl, b"co64")?) {
        (Some(stco), None) => chunk_offsets(stco.body, false, count)?,
        (None, Some(co64)) => chunk_offsets(co64.body, true, count)?,
        _ => return Err(Error::Unsupported("chunk offsets missing or stated twice")),
    };
    let per_chunk = samples_per_chunk(
        needed(&stbl, b"stsc", "no sample-to-chunk table")?.body,
        chunks.len(),
        count,
    )?;
    if let Some(stss) = only(&stbl, b"stss")? {
        first_is_sync(stss.body)?;
    }
    let mut samples = Vec::with_capacity(count);
    let mut index = 0;
    for (chunk, &in_chunk) in chunks.iter().zip(&per_chunk) {
        let mut offset = *chunk;
        for _ in 0..in_chunk {
            let len = sizes[index];
            let end = offset
                .checked_add(len as u64)
                .ok_or(Error::Truncated("sample past the file"))?;
            if end > file_len as u64 || len == 0 {
                return Err(Error::Truncated("sample past the file"));
            }
            samples.push(Sample {
                offset: offset as usize,
                len,
                duration_ms: durations[index],
            });
            offset = end;
            index += 1;
        }
    }
    Ok(Read {
        track: Track {
            id,
            width,
            height,
            samples,
        },
        role,
        aux_for,
        repeats,
    })
}

fn track_id(tkhd: &[u8]) -> Result<u32, Error> {
    let (version, _, mut at) = full(tkhd)?;
    at.skip(if version == 1 { 16 } else { 8 }, "track header times")?;
    at.u32("track id")
}

/// The track an `auxl` reference names, if the track has one.
fn auxiliary_of(tref: &[u8]) -> Result<Option<u32>, Error> {
    let Some(auxl) = only(&children(tref)?, b"auxl")? else {
        return Ok(None);
    };
    Cursor::new(auxl.body).u32("auxl reference").map(Some)
}

fn timescale(mdhd: &[u8]) -> Result<u32, Error> {
    let (version, _, mut at) = full(mdhd)?;
    at.skip(if version == 1 { 16 } else { 8 }, "media header times")?;
    match at.u32("timescale")? {
        0 => Err(Error::Unsupported("a timescale of zero")),
        t => Ok(t),
    }
}

fn handler(hdlr: &[u8]) -> Result<[u8; 4], Error> {
    let (_, _, mut at) = full(hdlr)?;
    at.skip(4, "handler pre_defined")?;
    let mut kind = [0; 4];
    kind.copy_from_slice(at.take(4, "handler type")?);
    Ok(kind)
}

/// The one AV1 sample entry's frame size (`VisualSampleEntry`, §12.1.3).
fn sample_entry(stsd: &[u8]) -> Result<(u32, u32), Error> {
    let (_, _, mut at) = full(stsd)?;
    if at.u32("sample description count")? != 1 {
        return Err(Error::Unsupported("one sample description expected"));
    }
    let entries = children(at.rest())?;
    let [entry] = entries.as_slice() else {
        return Err(Error::Unsupported("one sample description expected"));
    };
    if &entry.kind != b"av01" {
        return Err(Error::Unsupported("not AV1"));
    }
    let mut at = Cursor::new(entry.body);
    // reserved (6), data_reference_index (2), pre_defined (2), reserved (2),
    // pre_defined (12), then the size.
    at.skip(24, "visual sample entry")?;
    let width = at.u16("width")?;
    let height = at.u16("height")?;
    Ok((u32::from(width), u32::from(height)))
}

/// Every sample's length (`stsz`), its count held to the bounds first.
fn sample_sizes(stsz: &[u8], bounds: &Bounds) -> Result<Vec<usize>, Error> {
    let (_, _, mut at) = full(stsz)?;
    let fixed = at.u32("sample size")?;
    let count = at.u32("sample count")?;
    if count == 0 || count > bounds.max_samples {
        return Err(Error::OutOfBounds("sample count"));
    }
    let count = count as usize;
    if fixed != 0 {
        return Ok(vec![fixed as usize; count]);
    }
    if at.rest().len() < count * 4 {
        return Err(Error::Truncated("sample sizes"));
    }
    (0..count)
        .map(|_| at.u32("sample size").map(|s| s as usize))
        .collect()
}

/// Every sample's duration in milliseconds (`stts`), which must account for
/// exactly `count` samples.
fn durations(stts: &[u8], count: usize, timescale: u32) -> Result<Vec<u32>, Error> {
    let (_, _, mut at) = full(stts)?;
    let entries = at.u32("time entries")? as usize;
    if entries > count || at.rest().len() < entries * 8 {
        return Err(Error::Truncated("time entries"));
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..entries {
        let run = at.u32("time run")? as usize;
        let delta = at.u32("time delta")?;
        if run > count - out.len() {
            return Err(Error::Unsupported("times for more samples than there are"));
        }
        let ms = (u64::from(delta) * 1000 / u64::from(timescale)).min(u64::from(u32::MAX)) as u32;
        out.extend(std::iter::repeat_n(ms.max(MIN_FRAME_MS), run));
    }
    if out.len() != count {
        return Err(Error::Unsupported("times for fewer samples than there are"));
    }
    Ok(out)
}

fn chunk_offsets(body: &[u8], wide: bool, samples: usize) -> Result<Vec<u64>, Error> {
    let (_, _, mut at) = full(body)?;
    let count = at.u32("chunk count")? as usize;
    let width = if wide { 8 } else { 4 };
    // Every chunk holds at least one sample.
    if count == 0 || count > samples || at.rest().len() < count * width {
        return Err(Error::Truncated("chunk offsets"));
    }
    (0..count)
        .map(|_| {
            if wide {
                at.u64("chunk offset")
            } else {
                at.u32("chunk offset").map(u64::from)
            }
        })
        .collect()
}

/// How many samples each chunk holds (`stsc`, §8.7.4), which must add up to
/// exactly `samples`.
fn samples_per_chunk(stsc: &[u8], chunks: usize, samples: usize) -> Result<Vec<u32>, Error> {
    let (_, _, mut at) = full(stsc)?;
    let entries = at.u32("chunk entries")? as usize;
    if entries == 0 || entries > chunks || at.rest().len() < entries * 12 {
        return Err(Error::Truncated("chunk entries"));
    }
    let mut runs = Vec::with_capacity(entries);
    for _ in 0..entries {
        let first = at.u32("first chunk")? as usize;
        let per = at.u32("samples per chunk")?;
        at.skip(4, "sample description index")?;
        runs.push((first, per));
    }
    let mut out = Vec::with_capacity(chunks);
    let mut total = 0usize;
    for (i, &(first, per)) in runs.iter().enumerate() {
        let next = runs.get(i + 1).map_or(chunks + 1, |r| r.0);
        // First chunks count from one, strictly rising, within the table.
        if first == 0 || first >= next || next > chunks + 1 || per == 0 {
            return Err(Error::Unsupported("a sample-to-chunk table out of order"));
        }
        for _ in first..next {
            total = total.saturating_add(per as usize);
            if total > samples {
                return Err(Error::Unsupported(
                    "chunks hold more samples than there are",
                ));
            }
            out.push(per);
        }
    }
    if out.len() != chunks || total != samples {
        return Err(Error::Unsupported(
            "chunks hold fewer samples than there are",
        ));
    }
    Ok(out)
}

/// A sequence that does not begin on a key frame cannot be started.
fn first_is_sync(stss: &[u8]) -> Result<(), Error> {
    let (_, _, mut at) = full(stss)?;
    if at.u32("sync entries")? == 0 || at.u32("first sync sample")? != 1 {
        return Err(Error::Unsupported("the first frame is not a key frame"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
