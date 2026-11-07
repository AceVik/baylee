//! Every decoder a socket reaches, fed what a socket could send: real frames of
//! real games, damaged, from a seeded stream (`docs/proposals/view-patches.md`
//! §"Sicherheit").
//!
//! ```text
//! target/tools/engine_perf fuzz --rounds 20000000
//! ```
//!
//! Random bytes die on the first tag. Real frames with a few bytes changed get
//! deep into a decoder, past the head of a patch and into its objects, which is
//! where a length or an index is believed. Nothing here is expected to read: what
//! is held is that nothing panics, overflows or runs away, and that whatever does
//! read goes on through what a client does with it (`apply`, a digest, JSON).
//! A panic stops the run, and it prints the seed and the round to play it again.

use baylee_core::wire::{decode, encode};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::PlayerView;
use baylee_protocol::v1::Envelope;
use baylee_protocol::v1::envelope::Msg;
use baylee_view::{
    GameStatic, LogTail, MorePrints, SeatSetting, StaticReader, ViewFrame, ViewPatch, ViewReader,
    apply, digest,
};
use prost::Message as _;

/// The most bytes a round's input may reach: past the seat socket's 64 KiB.
const MOST: usize = 96 * 1024;

/// A stream of numbers that is the same on every run of a seed.
struct Stream(u64);

impl Stream {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound.max(1) as u64).unwrap_or(0)
    }
}

/// Real frames, by what reads them.
#[derive(Default)]
pub struct Corpus {
    /// Whole views, and the view each stream of patches starts from.
    snapshots: Vec<Vec<u8>>,
    patches: Vec<Vec<u8>>,
    questions: Vec<Vec<u8>>,
    logs: Vec<Vec<u8>>,
    statics: Vec<Vec<u8>>,
    more: Vec<Vec<u8>>,
    actions: Vec<Vec<u8>>,
    envelopes: Vec<Vec<u8>>,
}

/// How many frames of a kind are worth keeping.
const KEEP: usize = 4096;

fn keep(list: &mut Vec<Vec<u8>>, bytes: &[u8]) {
    if list.len() < KEEP && !bytes.is_empty() {
        list.push(bytes.to_vec());
    }
}

impl Corpus {
    /// Takes in one frame a seat was sent.
    pub fn frame(&mut self, frame: &Envelope) {
        keep(&mut self.envelopes, &frame.encode_to_vec());
        match &frame.msg {
            Some(Msg::StateDelta(delta)) => {
                if matches!(
                    ViewFrame::decode(&delta.view_frame),
                    Ok(ViewFrame::Snapshot(_))
                ) {
                    keep(&mut self.snapshots, &delta.view_frame);
                } else {
                    keep(&mut self.patches, &delta.view_frame);
                }
                keep(&mut self.logs, &delta.log);
            }
            Some(Msg::ChoiceRequest(choice)) => keep(&mut self.questions, &choice.pending),
            Some(Msg::GameStatic(statics)) => {
                keep(&mut self.statics, &statics.statics);
                keep(&mut self.more, &statics.more);
            }
            _ => {}
        }
    }

    /// Takes in one action a seat answered with.
    pub fn action(&mut self, action: &PlayerAction) {
        keep(&mut self.actions, &encode(action));
    }

    /// Takes in another game's frames.
    pub fn merge(&mut self, other: Self) {
        self.snapshots.extend(other.snapshots);
        self.patches.extend(other.patches);
        self.questions.extend(other.questions);
        self.logs.extend(other.logs);
        self.statics.extend(other.statics);
        self.more.extend(other.more);
        self.actions.extend(other.actions);
        self.envelopes.extend(other.envelopes);
    }

    fn kinds(&self) -> [&[Vec<u8>]; 8] {
        [
            &self.snapshots,
            &self.patches,
            &self.questions,
            &self.logs,
            &self.statics,
            &self.more,
            &self.actions,
            &self.envelopes,
        ]
    }
}

/// A varint of a number a frame has no business claiming.
fn huge_length(stream: &mut Stream) -> Vec<u8> {
    let mut value = stream.next() >> stream.below(40);
    let mut out = Vec::new();
    while value >= 0x80 {
        out.push(u8::try_from(value & 0x7f).unwrap_or(0) | 0x80);
        value >>= 7;
    }
    out.push(u8::try_from(value).unwrap_or(0));
    out
}

/// One frame, damaged a few ways.
fn damaged(stream: &mut Stream, frame: &[u8], other: &[u8]) -> Vec<u8> {
    let mut bytes = frame.to_vec();
    for _ in 0..=stream.below(4) {
        let at = stream.below(bytes.len());
        match stream.below(8) {
            0 => {
                if let Some(byte) = bytes.get_mut(at) {
                    *byte ^= 1 << stream.below(8);
                }
            }
            1 => {
                if let Some(byte) = bytes.get_mut(at) {
                    *byte = [0, 1, 0x7f, 0x80, 0xff][stream.below(5)];
                }
            }
            2 => bytes.truncate(at),
            3 => {
                let end = (at + stream.below(32)).min(bytes.len());
                bytes.drain(at..end);
            }
            4 => {
                let length = huge_length(stream);
                bytes.splice(at..at, length);
            }
            5 => {
                let from = stream.below(other.len());
                let end = (from + stream.below(256)).min(other.len());
                bytes.splice(at..at, other[from..end].iter().copied());
            }
            6 => {
                let end = (at + stream.below(64)).min(bytes.len());
                let copy = bytes[at..end].to_vec();
                bytes.splice(at..at, copy);
            }
            _ => {
                let run = stream.below(16);
                let fill = u8::try_from(stream.next() & 0xff).unwrap_or(0);
                bytes.splice(at..at, std::iter::repeat_n(fill, run));
            }
        }
        bytes.truncate(MOST);
    }
    bytes
}

/// What a seat's reader holds between frames.
#[derive(Default)]
struct Readers {
    views: ViewReader,
    statics: StaticReader,
}

/// What each decoder read, of the rounds put to it.
#[derive(Default)]
struct Reads {
    views: u64,
    patches: u64,
    applied: u64,
    actions: u64,
    questions: u64,
    logs: u64,
    statics: u64,
    more: u64,
    settings: u64,
    envelopes: u64,
}

/// Everything a client or an engine would do with `bytes`.
fn read(bytes: &[u8], readers: &mut Readers, base: &PlayerView, reads: &mut Reads) {
    if let Ok(frame) = ViewFrame::decode(bytes) {
        reads.views += 1;
        let _ = frame.seat();
        if let ViewFrame::Snapshot(view) = &frame {
            std::hint::black_box(digest(view));
            let _ = std::hint::black_box(serde_json::to_vec(view));
        }
    }
    // Past the tag of a frame, or a damaged patch would rarely read as one.
    if let Some(Ok(patch)) = bytes.get(1..).map(decode::<ViewPatch>) {
        reads.patches += 1;
        let mut aimed = patch;
        // Past the two checks a made-up patch would die on, so that what it
        // says is really applied to a real view.
        aimed.seat = base.seat;
        aimed.base = base.seq;
        aimed.digest = digest(base);
        reads.applied += u64::from(apply(base, &aimed).is_ok());
    }
    let _ = readers.views.read(bytes);
    let _ = readers.statics.read(&[], bytes);
    let _ = readers.statics.read(bytes, &[]);
    reads.actions += u64::from(decode::<PlayerAction>(bytes).is_ok());
    if let Ok(pending) = decode::<Pending>(bytes) {
        reads.questions += 1;
        let _ = std::hint::black_box(serde_json::to_vec(&pending));
    }
    reads.logs += u64::from(decode::<LogTail>(bytes).is_ok());
    reads.statics += u64::from(decode::<GameStatic>(bytes).is_ok());
    reads.more += u64::from(decode::<MorePrints>(bytes).is_ok());
    reads.settings += u64::from(decode::<SeatSetting>(bytes).is_ok());
    reads.envelopes += u64::from(Envelope::decode(bytes).is_ok());
}

/// Damages the corpus `rounds` times from `seed`, and reads every result with
/// every decoder.
pub fn run(corpus: &Corpus, rounds: u64, seed: u64) {
    let mut stream = Stream(seed | 1);
    let kinds = corpus.kinds();
    let base: PlayerView = corpus
        .snapshots
        .iter()
        .find_map(|frame| ViewFrame::whole(frame))
        .expect("a game opens on a whole view");
    let whole = corpus
        .snapshots
        .first()
        .expect("a game opens on a whole view");
    for (name, kind) in [
        "snapshots",
        "patches",
        "questions",
        "logs",
        "statics",
        "more",
        "actions",
        "envelopes",
    ]
    .iter()
    .zip(kinds)
    {
        println!("corpus {name}: {} frames", kind.len());
    }
    let mut readers = Readers::default();
    let mut reads = Reads::default();
    let started = std::time::Instant::now();
    for round in 0..rounds {
        let kind = kinds[stream.below(kinds.len())];
        let (Some(frame), Some(other)) = (
            kind.get(stream.below(kind.len())),
            kinds[stream.below(kinds.len())].first(),
        ) else {
            continue;
        };
        let bytes = damaged(&mut stream, frame, other);
        if round % 1024 == 0 {
            let _ = readers.views.read(whole);
            if let Some(statics) = corpus.statics.first() {
                let _ = readers.statics.read(statics, &[]);
            }
        }
        read(&bytes, &mut readers, &base, &mut reads);
        if round % (1 << 22) == 0 {
            eprintln!("fuzz seed={seed} round={round}");
        }
    }
    println!(
        "fuzz seed={seed}: {rounds} rounds in {:.0} s, no panic; read as a frame {}, a patch {} ({} applied), an action {}, a question {}, a log {}, a payload {}, printings {}, a setting {}, an envelope {}",
        started.elapsed().as_secs_f64(),
        reads.views,
        reads.patches,
        reads.applied,
        reads.actions,
        reads.questions,
        reads.logs,
        reads.statics,
        reads.more,
        reads.settings,
        reads.envelopes,
    );
}
