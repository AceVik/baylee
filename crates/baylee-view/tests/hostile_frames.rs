//! What a hostile frame can make a reader do, in the real decoder, against a
//! counting allocator: not panic, and not allocate on a length it merely claims.
//!
//! A client's frames come from whoever it connected to, and the gateway forwards
//! them unread. `baylee_core::wire` says what must hold; this measures it.

#![allow(clippy::cast_possible_truncation)] // bytes cut from words on purpose

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use baylee_core::wire;
use baylee_view::{PlayerView, ViewFrame, ViewPatch, ViewReader};

/// Tracks the bytes held and the most held since the last reset.
struct Counting;

static HELD: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call goes straight to `System` with the arguments it was given,
// so the allocator's contract is `System`'s. The counters are plain atomics and
// allocate nothing.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let held = HELD.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK.fetch_max(held, Ordering::Relaxed);
        // SAFETY: the caller's layout is passed on unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HELD.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: `ptr` came from `System.alloc` with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The most `run` held above what was held when it started.
fn peak_of(run: impl FnOnce()) -> usize {
    let before = HELD.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    run();
    PEAK.load(Ordering::Relaxed).saturating_sub(before)
}

/// `value` as `postcard` writes a length: seven bits a byte, low first.
fn varint(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    while value >= 0x80 {
        out.push((value & 0x7f) as u8 | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
    out
}

/// The most anything here may hold while refusing a frame: `serde` caps what a
/// list reserves from a claim at 1 MiB, and a reader holds a little beside it.
const BOUND: usize = 2 << 20;

/// A patch frame whose first list claims `claim` elements and then stops.
fn claiming(claim: u64) -> Vec<u8> {
    // Variant 1 is a patch; then seat, base, seq, and the `zones` length.
    let mut frame = vec![1, 0, 1, 2];
    frame.extend(varint(claim));
    frame
}

#[test]
fn a_claimed_length_allocates_nothing_for_itself() {
    for claim in [
        1 << 20,
        1 << 32,
        1 << 40,
        u64::from(u32::MAX),
        u64::MAX >> 1,
        u64::MAX,
    ] {
        let frame = claiming(claim);
        let mut read = None;
        let held = peak_of(|| read = Some(ViewFrame::decode(&frame).is_err()));
        assert_eq!(
            read,
            Some(true),
            "a claim of {claim} with nothing behind it"
        );
        assert!(held < BOUND, "a claim of {claim} held {held} bytes");
    }
}

/// The same claim with 60 KiB of real bytes behind it: the frame a socket's
/// limit lets through, so the reader has something to chew on.
#[test]
fn a_claimed_length_at_the_frame_limit_holds_no_more_than_the_frame_is_worth() {
    let mut frame = claiming(1 << 40);
    frame.resize(60 * 1024, 0);
    let held = peak_of(|| assert!(ViewFrame::decode(&frame).is_err()));
    assert!(held < 16 * BOUND, "60 KiB of frame held {held} bytes");
}

/// A string that claims more than there is is refused before anything is read.
#[test]
fn a_claimed_string_is_checked_against_what_is_left() {
    let mut frame = varint(1 << 40);
    frame.extend(b"short");
    let held = peak_of(|| assert!(wire::decode::<String>(&frame).is_err()));
    assert!(held < 4096, "a claimed string held {held} bytes");
}

#[test]
fn a_reader_holds_its_view_through_a_hostile_frame() {
    let mut reader = ViewReader::default();
    for claim in [1 << 40, u64::MAX] {
        assert!(reader.read(&claiming(claim)).is_err() || reader.view().is_none());
    }
    assert!(reader.view().is_none());
}

/// Every prefix of real frames, and every single byte of them changed: refused
/// or read, never a panic.
#[test]
fn every_cut_and_every_flipped_byte_of_a_frame_is_survived() {
    let view: PlayerView =
        serde_json::from_str(include_str!("hostile_view.json")).expect("the sample view reads");
    let mut next = view.clone();
    next.seq += 1;
    next.turn += 1;
    next.battlefield.pop();
    let frames = [
        ViewFrame::snapshot_of(&view),
        wire::encode(&ViewFrame::Patch(Box::new(baylee_view::diff(&view, &next)))),
    ];
    for frame in &frames {
        for cut in 0..frame.len() {
            let mut reader = ViewReader::default();
            let _ = reader.read(&frame[..cut]);
        }
        for at in 0..frame.len() {
            for flip in [0x01, 0x80, 0xff] {
                let mut damaged = frame.clone();
                damaged[at] ^= flip;
                let mut reader = ViewReader::default();
                let _ = reader.read(&frames[0]);
                let _ = reader.read(&damaged);
            }
        }
    }
}

/// A patch made up byte by byte from a seeded stream: as many as a test can
/// afford, each read as a frame on a view that is really held.
#[test]
fn a_million_made_up_frames_are_survived() {
    let view: PlayerView =
        serde_json::from_str(include_str!("hostile_view.json")).expect("the sample view reads");
    let whole = ViewFrame::snapshot_of(&view);
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut reader = ViewReader::default();
    reader.read(&whole).expect("a whole view reads");
    let mut bytes = Vec::with_capacity(256);
    for _ in 0..1_000_000 {
        bytes.clear();
        let length = (next() % 192) as usize;
        // Mostly the shape of a patch, so a good part of these get past the head.
        bytes.extend([1, 0]);
        bytes.extend((0..length).map(|_| next() as u8));
        if let Ok(patch) = wire::decode::<ViewPatch>(&bytes[1..]) {
            let _ = baylee_view::apply(&view, &patch);
        }
        let _ = reader.read(&bytes);
        if reader.view() != Some(&view) || bytes.first() == Some(&0) {
            let _ = reader.read(&whole);
        }
    }
    assert!(reader.read(&whole).is_ok());
}

/// The sample view with a spell on the stack that divides its damage
/// (`PlayerView::divided`, protocol 44) among two objects and a player.
fn with_a_division(shares: Vec<(baylee_view::TargetRef, u32)>) -> PlayerView {
    let mut view: PlayerView =
        serde_json::from_str(include_str!("hostile_view.json")).expect("the sample view reads");
    let spell = view.stack.first_mut().expect("the sample has a stack");
    // The same targets whatever the division, so a frame differs only there.
    spell.targets = three_shares().iter().map(|(target, _)| *target).collect();
    let object = spell.id;
    view.divided = if shares.is_empty() {
        Vec::new()
    } else {
        vec![baylee_view::Division { object, shares }]
    };
    view
}

fn three_shares() -> Vec<(baylee_view::TargetRef, u32)> {
    use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};
    use baylee_view::TargetRef;
    let object = |slot| {
        TargetRef::Object(DamageSourceRef {
            object: ObjectId::new(slot, 0),
            version: 0,
        })
    };
    vec![
        (object(124), 1),
        (object(125), 1),
        (TargetRef::Player(PlayerId::new(1)), 3),
    ]
}

/// A divided spell goes whole and as a patch, and reads as it was written;
/// every cut and every flipped byte of either is refused or read, never a
/// panic, and a refusal leaves the view held.
#[test]
fn a_division_on_the_stack_survives_the_wire_and_every_damage() {
    let view = with_a_division(three_shares());
    let whole = ViewFrame::snapshot_of(&view);
    let mut reader = ViewReader::default();
    reader.read(&whole).expect("a whole view reads");
    assert_eq!(reader.view().map(|v| &v.divided), Some(&view.divided));
    let mut next = view.clone();
    next.seq += 1;
    next.divided[0].shares[2].1 = 4;
    let patch = wire::encode(&ViewFrame::Patch(Box::new(baylee_view::diff(&view, &next))));
    reader.read(&patch).expect("the patch reads");
    assert_eq!(reader.view(), Some(&next));
    for frame in [&whole, &patch] {
        for cut in 0..frame.len() {
            let mut reader = ViewReader::default();
            let _ = reader.read(&frame[..cut]);
        }
        for at in 0..frame.len() {
            for flip in [0x01, 0x80, 0xff] {
                let mut damaged = frame.clone();
                damaged[at] ^= flip;
                let mut reader = ViewReader::default();
                let _ = reader.read(&whole);
                if reader.read(&damaged).is_err() {
                    assert_eq!(reader.view(), Some(&view), "a refusal keeps the view held");
                }
            }
        }
    }
}

/// A list of divisions, or one division's shares, that claims more than the
/// frame holds allocates nothing for the claim. Where each list stands is
/// found by writing it empty and with one entry: the first byte that differs
/// is its length.
#[test]
fn a_claimed_division_allocates_nothing_for_the_claim() {
    let share = (
        baylee_view::TargetRef::Player(baylee_core::ids::PlayerId::new(1)),
        3,
    );
    let none = ViewFrame::snapshot_of(&with_a_division(Vec::new()));
    let one = ViewFrame::snapshot_of(&with_a_division(vec![share]));
    let mut no_shares = with_a_division(vec![share]);
    no_shares.divided[0].shares.clear();
    let no_shares = ViewFrame::snapshot_of(&no_shares);
    let first_difference = |a: &[u8], b: &[u8]| {
        a.iter()
            .zip(b)
            .position(|(x, y)| x != y)
            .expect("the two frames differ")
    };
    let list = first_difference(&none, &one);
    assert_eq!((none[list], one[list]), (0, 1), "the divisions' length");
    let shares = first_difference(&no_shares, &one);
    assert_eq!(
        (no_shares[shares], one[shares]),
        (0, 1),
        "the shares' length"
    );
    for head in [&none[..list], &no_shares[..shares]] {
        for claim in [1 << 20, 1 << 40, u64::MAX] {
            let mut frame = head.to_vec();
            frame.extend(varint(claim));
            frame.resize(60 * 1024, 0x01);
            let held = peak_of(|| assert!(ViewFrame::decode(&frame).is_err()));
            assert!(held < 16 * BOUND, "a claim of {claim} held {held} bytes");
        }
    }
}

/// A log line's step (`LogEntry::step`, protocol 44) goes and reads as it was
/// written, and every cut and flipped byte of a log frame that carries one is
/// refused or read, never a panic. A step is one of twelve: a byte that names
/// no step is refused.
#[test]
fn a_log_line_s_step_survives_the_wire_and_every_damage() {
    use baylee_core::ids::PlayerId;
    use baylee_view::{LogEntry, LogEvent, LogTail, Phase, Step};
    let entry = |step: Step| LogEntry {
        turn: 25,
        repeat: 1,
        at: 0,
        event: LogEvent::Shuffled {
            player: PlayerId::new(0),
        },
        phase: Some(Phase::Combat),
        step: Some(step),
    };
    let tail = LogTail {
        from: 40,
        entries: vec![entry(Step::CombatDamageFirst), entry(Step::CombatDamage)],
    };
    let frame = wire::encode(&tail);
    let read: LogTail = wire::decode(&frame).expect("the tail reads");
    assert_eq!(read, tail);
    for cut in 0..frame.len() {
        assert!(
            wire::decode::<LogTail>(&frame[..cut]).is_err(),
            "a cut at {cut} read"
        );
    }
    for at in 0..frame.len() {
        for flip in [0x01, 0x80, 0xff] {
            let mut damaged = frame.clone();
            damaged[at] ^= flip;
            let _ = wire::decode::<LogTail>(&damaged);
        }
    }
    let mut unnamed = frame.clone();
    let last_step = unnamed.len() - 1;
    unnamed[last_step] = 12;
    assert!(
        wire::decode::<LogTail>(&unnamed).is_err(),
        "the thirteenth step does not exist"
    );
}
