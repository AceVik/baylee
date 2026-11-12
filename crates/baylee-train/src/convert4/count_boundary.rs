//! The count boundary (commit 445a4e276): records written before it.
//!
//! From that commit on, "for each creature, its controller sacrifices a
//! permanent of their choice unless they pay {1}" (Fade Away; in the engine,
//! a `ForEachMatching` whose instructions sacrifice) counts the creatures as
//! it begins, asks the active player's payments and sacrifices before any
//! other player's, and sacrifices everything chosen at once
//! (`docs/engine-internals.md` §"For each permanent, its controller"). A
//! build before it asked creature by creature in battlefield order and
//! sacrificed each choice at once. The count is in the snapshot hash while
//! it is open, so a record written before the boundary parts from a later
//! build as such an instruction begins to resolve, and the later build asks
//! other players other questions from there: the game itself goes another
//! way. Such a record is converted by the runtime that wrote it, so the
//! audit names the boundary instead of a divergence.
//! `engine::recorded_lockstep_tests` (with `BAYLEE_LOCKSTEP_BEFORE=for-each`)
//! finds the records a set holds that cross it.

use super::{Line, PlayerId};
use baylee_engine::engine::Engine;
use baylee_gamehost::RegistryLookup;

/// The commit from which the engine counts first.
pub const COUNT_BOUNDARY: &str = "445a4e276";

/// How a refusal of a record from before [`COUNT_BOUNDARY`] begins, when it
/// opens with this build's hash: the conversion stops on it rather than
/// leaving the game out, and `--llm-bc-skip-unreplayable` does not take it.
pub const COUNT_REFUSAL: &str = "record written before count boundary 445a4e276:";

/// The rule a refusal names.
pub const COUNT_RULE: &str =
    "convert records written before 445a4e276 with the runtime that wrote them";

/// Which digest a record is held to: the engine hash (an exact audit) or
/// the public table (`seen`, a record of another build).
#[derive(Clone, Copy)]
enum Digest {
    Exact,
    Seen,
}

/// Where a record from before the boundary parts from this build.
struct Crossing {
    build: String,
    /// The input after which this build parts from the record, or which
    /// this build refuses.
    first: u64,
}

impl Crossing {
    fn what(&self) -> String {
        format!(
            "from input {}, it resolves \"for each creature, its controller sacrifices …\" \
             (Fade Away) creature by creature in battlefield order, where this build counts \
             first, asks in APNAP order and sacrifices together",
            self.first
        )
    }
}

fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// Plays `record` on this build, held to `digest`, and finds where it
/// crosses the boundary; `None` when it departs from this build anywhere
/// else, or nowhere.
///
/// One departure is the boundary's: the first input after which the
/// record's digest is not this build's, or that this build refuses, falls
/// while this build holds a count open (`Engine::count_open`), before the
/// input or after it. Up to there the record is this build's game; from
/// there the two builds ask different questions.
fn crossing(record: &[u8], digest: Digest) -> Option<Crossing> {
    let mut lines = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(serde_json::from_slice::<Line>);
    let Some(Ok(Line::Header {
        build,
        preset,
        hash,
        seen,
        ..
    })) = lines.next()
    else {
        return None;
    };
    let of = |engine: &Engine<RegistryLookup>| match digest {
        Digest::Exact => hex(engine.snapshot_hash()),
        Digest::Seen => hex(baylee_gamehost::record::seen(engine.state())),
    };
    let mut engine = Engine::new(&preset, RegistryLookup).ok()?;
    let opening = match digest {
        Digest::Exact => Some(hash),
        Digest::Seen => seen,
    };
    if opening.as_deref() != Some(of(&engine).as_str()) {
        return None;
    }
    for line in lines {
        let Line::Input {
            n,
            seat,
            action,
            hash,
            seen,
            ..
        } = line.ok()?
        else {
            continue;
        };
        let wrote = match digest {
            Digest::Exact => Some(hash),
            Digest::Seen => seen,
        };
        let open = engine.count_open();
        let parted = match engine.apply(PlayerId::new(seat), action) {
            Err(_) => true,
            Ok(()) => wrote.as_deref() != Some(of(&engine).as_str()),
        };
        if parted {
            return (open || engine.count_open()).then_some(Crossing { build, first: n });
        }
    }
    None
}

/// Why a record that opens with this build's hash and whose exact replay
/// diverged or was refused is one written before [`COUNT_BOUNDARY`], or
/// `None` when it departs for anything else.
pub(super) fn refusal(record: &[u8]) -> Option<String> {
    let found = crossing(record, Digest::Exact)?;
    Some(format!(
        "{COUNT_REFUSAL} written by {build}, converted by {this}: {what}; {COUNT_RULE}",
        build = found.build,
        this = baylee_gamehost::record::this_build(),
        what = found.what(),
    ))
}

/// The same for a record of another build, held to its public table: the
/// reason a seen replay gives, so that it stays one
/// `--llm-bc-skip-unreplayable` may leave out, as for any rule this build
/// plays differently.
pub(super) fn seen_refusal(record: &[u8]) -> Option<String> {
    let found = crossing(record, Digest::Seen)?;
    Some(format!(
        "seen replay audit: written before count boundary {COUNT_BOUNDARY} by {build}: \
         {what}; {COUNT_RULE}",
        build = found.build,
        what = found.what(),
    ))
}
