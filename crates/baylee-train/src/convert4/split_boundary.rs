//! The split boundary (commit 7c879ff33): records written before it.
//!
//! From that commit on, the engine reads a split card whose halves are
//! instants and sorceries as both halves everywhere but the stack (CR 709.4:
//! Fire // Ice in a hand is a blue and red card of mana value 4), where a
//! build before it read the left half alone (`docs/engine-internals.md`
//! §"A split card is both of its halves"). Every object's characteristics
//! are in the snapshot hash, so a record whose decks hold such a card opens
//! with another hash than this build's, whether or not the game ever asked
//! about the card, and its exact replay diverges before the first input.
//! Such a record is converted by the runtime that wrote it, so the audit
//! names the boundary instead of a divergence. One of another build replays
//! by its public table as before (`seen` reads cards by index and no
//! characteristic), and departs from it only where the card's colours or
//! mana value changed an answer. `engine::recorded_lockstep_tests` (with
//! `BAYLEE_LOCKSTEP_BEFORE=split`) finds the records a set holds that cross
//! it.

use super::Line;
use baylee_engine::engine::Engine;
use baylee_gamehost::RegistryLookup;

/// The commit from which the engine reads a split card as both halves.
pub const SPLIT_BOUNDARY: &str = "7c879ff33";

/// How a refusal of a record from before [`SPLIT_BOUNDARY`] begins: the
/// conversion stops on it rather than leaving the game out, and
/// `--llm-bc-skip-unreplayable` does not take it.
pub const SPLIT_REFUSAL: &str = "record written before split boundary 7c879ff33:";

/// The rule a refusal names.
pub const SPLIT_RULE: &str =
    "convert records written before 7c879ff33 with the runtime that wrote them";

fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// Why a record whose exact replay diverged at its opening is one written
/// before [`SPLIT_BOUNDARY`], or `None` when it opens otherwise.
///
/// A build from before the boundary opened the same game with this build's
/// hash, had this build read every split card by its left half
/// (`Engine::snapshot_hash_reading_split_cards_by_left_half`); a record
/// whose opening hash is that one, and not this build's, is one such a build
/// wrote. The record's build string names a version and a commit, which
/// says nothing of where that commit stands to the boundary, so the hash
/// decides.
pub(super) fn refusal(record: &[u8]) -> Option<String> {
    let header = record.split(|&b| b == b'\n').find(|l| !l.is_empty())?;
    let Ok(Line::Header {
        build,
        preset,
        hash: opening,
        ..
    }) = serde_json::from_slice::<Line>(header)
    else {
        return None;
    };
    let engine = Engine::new(&preset, RegistryLookup).ok()?;
    if opening == hex(engine.snapshot_hash())
        || opening != hex(engine.snapshot_hash_reading_split_cards_by_left_half())
    {
        return None;
    }
    let mut split: Vec<&str> = preset
        .seats
        .iter()
        .flat_map(|seat| {
            seat.deck
                .iter()
                .chain(&seat.sideboard)
                .chain(&seat.commanders)
                .chain(&seat.starting_battlefield)
                .chain(seat.starting_hand.iter().flatten())
        })
        .filter_map(|entry| baylee_cards::by_index(entry.card))
        .filter(|def| def.is_split_spell_card())
        .filter_map(|def| baylee_cards::decks::whole_name(def.index))
        .collect();
    split.sort_unstable();
    split.dedup();
    Some(format!(
        "{SPLIT_REFUSAL} written by {build}, converted by {this}: its decks hold {cards}, \
         a split card this build reads as both halves off the stack (CR 709.4) and that \
         build by its left half, so it opens with another hash; {SPLIT_RULE}",
        this = baylee_gamehost::record::this_build(),
        cards = split.join(", "),
    ))
}
