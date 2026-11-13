//! The reader against files built here box by box, so each refusal is
//! reached by exactly the field it names, and against a sequence ffmpeg made.

use super::*;

#[allow(clippy::trivially_copy_pass_by_ref)] // a box's type is written `b"moov"`
fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    out
}

#[allow(clippy::trivially_copy_pass_by_ref)] // a box's type is written `b"moov"`
fn full_box(kind: &[u8; 4], version_flags: u32, body: &[u8]) -> Vec<u8> {
    let mut inner = version_flags.to_be_bytes().to_vec();
    inner.extend_from_slice(body);
    boxed(kind, &inner)
}

fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}

/// What a test file's one or two tracks say.
#[derive(Clone)]
struct Spec {
    id: u32,
    handler: [u8; 4],
    aux_for: Option<u32>,
    side: (u16, u16),
    sizes: Vec<u32>,
    /// `stts` runs: (count, delta in the timescale).
    times: Vec<(u32, u32)>,
    timescale: u32,
    /// `stsc` runs: (first chunk, samples per chunk).
    chunks: Vec<(u32, u32)>,
    chunk_count: usize,
    repeats: bool,
}

impl Spec {
    fn colour(frames: u32) -> Self {
        Self {
            id: 1,
            handler: *b"pict",
            aux_for: None,
            side: (256, 256),
            sizes: vec![10; frames as usize],
            times: vec![(frames, 100)],
            timescale: 1000,
            chunks: vec![(1, frames)],
            chunk_count: 1,
            repeats: true,
        }
    }

    fn trak(&self, chunk_offsets: &[u32]) -> Vec<u8> {
        let mut tkhd = vec![0; 8];
        tkhd.extend(words(&[self.id]));
        tkhd.extend(vec![0; 60]);
        let mut entry = vec![0; 24];
        entry.extend_from_slice(&self.side.0.to_be_bytes());
        entry.extend_from_slice(&self.side.1.to_be_bytes());
        entry.extend(vec![0; 50]);
        entry.extend(boxed(b"av1C", &[0x81, 0, 0, 0]));
        let mut stsd = words(&[1]);
        stsd.extend(boxed(b"av01", &entry));
        let mut stts = words(&[self.times.len() as u32]);
        for &(n, d) in &self.times {
            stts.extend(words(&[n, d]));
        }
        let mut stsc = words(&[self.chunks.len() as u32]);
        for &(first, per) in &self.chunks {
            stsc.extend(words(&[first, per, 1]));
        }
        let mut stsz = words(&[0, self.sizes.len() as u32]);
        stsz.extend(words(&self.sizes));
        let mut stco = words(&[chunk_offsets.len() as u32]);
        stco.extend(words(chunk_offsets));
        let stbl = [
            full_box(b"stsd", 0, &stsd),
            full_box(b"stts", 0, &stts),
            full_box(b"stsc", 0, &stsc),
            full_box(b"stsz", 0, &stsz),
            full_box(b"stco", 0, &stco),
        ]
        .concat();
        let minf = boxed(b"stbl", &stbl);
        let mut hdlr = words(&[0]);
        hdlr.extend_from_slice(&self.handler);
        hdlr.extend(vec![0; 13]);
        let mut mdhd = vec![0; 8];
        mdhd.extend(words(&[self.timescale, 0]));
        mdhd.extend(vec![0; 4]);
        let mdia = [
            full_box(b"mdhd", 0, &mdhd),
            full_box(b"hdlr", 0, &hdlr),
            boxed(b"minf", &minf),
        ]
        .concat();
        let mut trak = full_box(b"tkhd", 0, &tkhd);
        if let Some(of) = self.aux_for {
            trak.extend(boxed(b"tref", &boxed(b"auxl", &words(&[of]))));
        }
        let elst = full_box(b"elst", u32::from(self.repeats), &words(&[0]));
        trak.extend(boxed(b"edts", &elst));
        trak.extend(boxed(b"mdia", &mdia));
        boxed(b"trak", &trak)
    }
}

/// A whole file: `ftyp`, `moov` and an `mdat` holding every sample's bytes,
/// each track's chunks laid one after another.
fn file(specs: &[Spec]) -> Vec<u8> {
    let ftyp = boxed(b"ftyp", b"avis\0\0\0\0avifmif1miaf");
    // The chunks' offsets depend on the moov's length, which does not
    // depend on their values: build once to measure, once for real.
    let build = |offsets: &[Vec<u32>]| {
        let traks: Vec<u8> = specs
            .iter()
            .zip(offsets)
            .flat_map(|(s, o)| s.trak(o))
            .collect();
        boxed(b"moov", &traks)
    };
    let zeros: Vec<Vec<u32>> = specs.iter().map(|s| vec![0; s.chunk_count]).collect();
    let moov_len = build(&zeros).len();
    let mut at = (ftyp.len() + moov_len + 8) as u32;
    let mut offsets = Vec::new();
    let mut payload = Vec::new();
    for spec in specs {
        let mut track_offsets = Vec::new();
        let mut sample = 0usize;
        for (i, &(first, per)) in spec.chunks.iter().enumerate() {
            let next = spec
                .chunks
                .get(i + 1)
                .map_or(spec.chunk_count as u32 + 1, |c| c.0);
            for _ in first..next {
                track_offsets.push(at);
                for _ in 0..per {
                    let len = spec.sizes.get(sample).copied().unwrap_or(0);
                    payload.extend(std::iter::repeat_n(0xAB, len as usize));
                    at += len;
                    sample += 1;
                }
            }
        }
        offsets.push(track_offsets);
    }
    [ftyp, build(&offsets), boxed(b"mdat", &payload)].concat()
}

#[test]
fn a_built_sequence_reads_back_frame_for_frame() {
    let bytes = file(&[Spec::colour(4)]);
    let seq = Sequence::read(&bytes, &Bounds::PICTURE).unwrap();
    assert_eq!((seq.colour.width, seq.colour.height), (256, 256));
    assert_eq!(seq.colour.samples.len(), 4);
    assert!(seq.repeats);
    assert!(seq.alpha.is_none());
    assert_eq!(seq.colour.duration_ms(), 400);
    for pair in seq.colour.samples.windows(2) {
        assert_eq!(pair[0].offset + pair[0].len, pair[1].offset);
    }
    let last = seq.colour.samples.last().unwrap();
    assert_eq!(last.offset + last.len, bytes.len());
    assert!(
        bytes[seq.colour.samples[0].offset..][..10]
            .iter()
            .all(|&b| b == 0xAB)
    );
}

#[test]
fn chunks_of_several_runs_place_every_sample() {
    let mut spec = Spec::colour(5);
    spec.sizes = vec![3, 4, 5, 6, 7];
    spec.chunks = vec![(1, 2), (3, 1)];
    spec.chunk_count = 3;
    let bytes = file(&[spec]);
    let seq = Sequence::read(&bytes, &Bounds::PICTURE).unwrap();
    let lens: Vec<usize> = seq.colour.samples.iter().map(|s| s.len).collect();
    assert_eq!(lens, [3, 4, 5, 6, 7]);
    for pair in seq.colour.samples.windows(2) {
        assert_eq!(pair[0].offset + pair[0].len, pair[1].offset);
    }
}

#[test]
fn an_alpha_track_is_kept_only_as_its_colour_tracks_partner() {
    let colour = Spec::colour(3);
    let mut alpha = Spec::colour(3);
    alpha.id = 2;
    alpha.handler = *b"auxv";
    alpha.aux_for = Some(1);
    let seq = Sequence::read(&file(&[colour.clone(), alpha.clone()]), &Bounds::PICTURE).unwrap();
    assert_eq!(seq.alpha.as_ref().map(|a| a.id), Some(2));

    let mut stranger = alpha.clone();
    stranger.aux_for = Some(7);
    let seq = Sequence::read(&file(&[colour.clone(), stranger]), &Bounds::PICTURE).unwrap();
    assert!(seq.alpha.is_none());

    let mut short = alpha;
    short.sizes = vec![10; 2];
    short.times = vec![(2, 100)];
    short.chunks = vec![(1, 2)];
    let seq = Sequence::read(&file(&[colour, short]), &Bounds::PICTURE).unwrap();
    assert!(seq.alpha.is_none());
}

#[test]
fn every_bound_refuses_the_file_that_passes_it() {
    let over = |spec: Spec| Sequence::read(&file(&[spec]), &Bounds::PICTURE).unwrap_err();

    let mut many = Spec::colour(101);
    many.sizes = vec![1; 101];
    assert_eq!(over(many), Error::OutOfBounds("sample count"));

    let mut long = Spec::colour(4);
    long.times = vec![(4, 2_600)];
    assert_eq!(over(long), Error::OutOfBounds("duration"));

    let mut wide = Spec::colour(1);
    wide.side = (257, 256);
    assert_eq!(over(wide), Error::OutOfBounds("frame size"));

    let mut small = Spec::colour(1);
    small.side = (128, 128);
    assert_eq!(
        over(small),
        Error::OutOfBounds("frame size is not the one allowed")
    );

    let big = vec![0; Bounds::PICTURE.max_bytes + 1];
    assert_eq!(
        Sequence::read(&big, &Bounds::PICTURE).unwrap_err(),
        Error::TooLarge(big.len())
    );
}

#[test]
fn tables_that_disagree_about_the_samples_are_refused() {
    let refused = |spec: Spec| Sequence::read(&file(&[spec]), &Bounds::PICTURE).is_err();

    let mut few_times = Spec::colour(4);
    few_times.times = vec![(3, 100)];
    assert!(refused(few_times));

    let mut many_times = Spec::colour(4);
    many_times.times = vec![(5, 100)];
    assert!(refused(many_times));

    let mut crowded = Spec::colour(4);
    crowded.chunks = vec![(1, 5)];
    assert!(refused(crowded));

    let mut backwards = Spec::colour(4);
    backwards.chunks = vec![(2, 2), (1, 2)];
    backwards.chunk_count = 2;
    assert!(refused(backwards));

    let mut empty_chunk = Spec::colour(4);
    empty_chunk.chunks = vec![(1, 0)];
    assert!(refused(empty_chunk));

    let mut stopped = Spec::colour(2);
    stopped.timescale = 0;
    assert!(refused(stopped));
}

#[test]
fn a_zero_duration_is_shown_for_the_shortest_frame() {
    let mut spec = Spec::colour(2);
    spec.times = vec![(2, 0)];
    let seq = Sequence::read(&file(&[spec]), &Bounds::PICTURE).unwrap();
    assert!(
        seq.colour
            .samples
            .iter()
            .all(|s| s.duration_ms == MIN_FRAME_MS)
    );
}

#[test]
fn a_sample_past_the_file_is_refused() {
    let mut bytes = file(&[Spec::colour(2)]);
    bytes.truncate(bytes.len() - 1);
    // The mdat now claims a byte it does not have.
    assert!(Sequence::read(&bytes, &Bounds::PICTURE).is_err());
}

#[test]
fn a_still_picture_is_not_a_sequence() {
    let mut bytes = file(&[Spec::colour(1)]);
    bytes[8..12].copy_from_slice(b"avif");
    bytes[16..20].copy_from_slice(b"avif");
    assert_eq!(
        Sequence::read(&bytes, &Bounds::PICTURE).unwrap_err(),
        Error::Unsupported("not an image sequence (no avis brand)")
    );
}

/// Every prefix of a valid file is refused or read, never a panic: the
/// cheapest fuzz there is, and it reaches every length check.
#[test]
fn every_prefix_and_every_flipped_byte_is_refused_or_read_without_a_panic() {
    let colour = Spec::colour(3);
    let mut alpha = Spec::colour(3);
    alpha.id = 2;
    alpha.handler = *b"auxv";
    alpha.aux_for = Some(1);
    let bytes = file(&[colour, alpha]);
    for end in 0..bytes.len() {
        let _ = Sequence::read(&bytes[..end], &Bounds::PICTURE);
    }
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    for _ in 0..4000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut copy = bytes.clone();
        let at = (seed as usize) % copy.len();
        copy[at] ^= (seed >> 32) as u8 | 1;
        if let Ok(seq) = Sequence::read(&copy, &Bounds::PICTURE) {
            for s in seq
                .colour
                .samples
                .iter()
                .chain(seq.alpha.iter().flat_map(|a| &a.samples))
            {
                assert!(s.offset + s.len <= copy.len());
            }
        }
    }
}
