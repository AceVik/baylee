//! The activator boundary (commit eb40dc593): records written before it.
//!
//! From that commit on, the engine lets go of the activator
//! (`activation_target_choice`) after a loyalty ability, and names none for
//! the special actions and the prepared cast `start_activation` also carries
//! (`docs/engine-internals.md` §"Forking a simulation"). The field is in the
//! snapshot hash, so a record written before the boundary carries other
//! hashes from the first such action on, until the next activation was
//! pressed. Its exact replay on a later build diverges there; the audit
//! names the boundary instead of a divergence, and the conversion stops,
//! because such a record is converted by the runtime that wrote it.
//! [`crossings`] finds the records a set holds that cross it, for deciding
//! which runtime converts which set.

use super::{Line, PlayerId};
use baylee_cards_dsl::AbilityDef;
use baylee_engine::choice::PlayerAction;
use baylee_engine::engine::Engine;
use baylee_gamehost::RegistryLookup;

/// The commit from which the engine lets go of the activator.
pub const ACTIVATOR_BOUNDARY: &str = "eb40dc593";

/// How a refusal of a record from before [`ACTIVATOR_BOUNDARY`] begins: the
/// conversion stops on it rather than leaving the game out, and
/// `--llm-bc-skip-unreplayable` does not take it.
pub const BOUNDARY_REFUSAL: &str = "record written before activator boundary eb40dc593:";

/// The rule a refusal names.
pub const BOUNDARY_RULE: &str =
    "convert records written before eb40dc593 with the runtime that wrote them";

fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// Why a record whose exact replay diverged is one written before
/// [`ACTIVATOR_BOUNDARY`], or `None` when it diverges for anything else.
///
/// A build from before the boundary wrote the same hash as this one at
/// every position except those where it still named an activator, and there
/// the hash it wrote is [`Engine::snapshot_hash_naming_stale_activator`] for
/// that seat. The record's build string names a version, a build number and
/// a commit, which says nothing of where that commit stands to the boundary,
/// so the hashes decide: every input must match this build's hash or that
/// one, and at least one must take the second.
pub(super) fn refusal(record: &[u8]) -> Option<String> {
    let mut lines = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(serde_json::from_slice::<Line>);
    let Some(Ok(Line::Header {
        build,
        preset,
        hash: opening,
        ..
    })) = lines.next()
    else {
        return None;
    };
    let mut engine = Engine::new(&preset, RegistryLookup).ok()?;
    if opening != hex(engine.snapshot_hash()) {
        return None;
    }
    let seats = u8::try_from(preset.seats.len()).ok()?;
    let (mut moved, mut first) = (0_u64, None);
    for line in lines {
        let Ok(Line::Input {
            n,
            seat,
            action,
            hash,
            ..
        }) = line
        else {
            continue;
        };
        engine.apply(PlayerId::new(seat), action).ok()?;
        if hash == hex(engine.snapshot_hash()) {
            continue;
        }
        let stale = (0..seats)
            .any(|s| hash == hex(engine.snapshot_hash_naming_stale_activator(PlayerId::new(s))));
        if !stale {
            return None;
        }
        moved += 1;
        first.get_or_insert(n);
    }
    let first = first?;
    Some(format!(
        "{BOUNDARY_REFUSAL} written by {build}, converted by {this}: {moved} of its engine \
         hashes differ from this build's, from input {first}, each only by the activator \
         a loyalty ability, a face-up, a door unlock or a prepared cast left standing; \
         {BOUNDARY_RULE}",
        this = baylee_gamehost::record::this_build(),
    ))
}

/// The actions in one record after which a build from before the boundary
/// still named an activator, by kind; all zero for a record the boundary
/// does not touch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Crossings {
    /// Loyalty abilities activated.
    pub loyalty: u32,
    /// Permanents turned face up.
    pub face_up: u32,
    /// Doors unlocked.
    pub door: u32,
    /// Prepared casts begun.
    pub prepared: u32,
}

impl Crossings {
    /// Whether the record crosses the boundary at all.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.loyalty + self.face_up + self.door + self.prepared > 0
    }
}

/// What `record` holds that crosses the boundary, read from its actions
/// alone, without playing them, so that it holds for a record of any build.
///
/// The three special indices are read off the action. A loyalty ability is
/// read off the card behind the activated object's id as the game began:
/// ids are dealt from the deck lists at setup and an object keeps its id
/// through its zone changes, so the card is the one the id names, and its
/// printed ability at that index is a loyalty ability. What that misses is
/// the loyalty ability of an object made during the game (a token, a copy
/// such as Spark Double entering as a planeswalker) and one granted by an
/// effect: such a game can cross unseen.
///
/// # Errors
/// A record whose header does not parse or whose preset builds no engine.
pub fn crossings(record: &[u8]) -> Result<Crossings, String> {
    let mut lines = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(serde_json::from_slice::<Line>);
    let Some(Ok(Line::Header { preset, .. })) = lines.next() else {
        return Err("no header".into());
    };
    let engine = Engine::new(&preset, RegistryLookup)
        .map_err(|_| "the preset builds no engine".to_owned())?;
    let state = engine.state();
    let mut found = Crossings::default();
    for line in lines {
        let Ok(Line::Input {
            action:
                PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                },
            ..
        }) = line
        else {
            continue;
        };
        if ability_index == baylee_engine::choice::TURN_FACE_UP {
            found.face_up += 1;
        } else if baylee_engine::choice::door_to_unlock(ability_index).is_some() {
            found.door += 1;
        } else if ability_index == baylee_engine::choice::PREPARED_CAST {
            found.prepared += 1;
        } else if state
            .printed_ability_list(source)
            .and_then(|list| list.abilities.get(ability_index as usize).copied())
            .is_some_and(|ability| matches!(ability, AbilityDef::Loyalty { .. }))
        {
            found.loyalty += 1;
        }
    }
    Ok(found)
}

/// Whether `record` opens with the hash this build gives its preset: the
/// records an exact audit plays on rather than refusing at the header, so
/// the only ones whose crossing a later `convert4` meets as [`refusal`]. One
/// that opens otherwise is another build's to this one either way (`seen`
/// for an LLM-BC conversion, a divergence for every other).
#[must_use]
pub fn opens_as_this_build(record: &[u8]) -> bool {
    let header = record
        .split(|&b| b == b'\n')
        .find(|l| !l.is_empty())
        .and_then(|l| serde_json::from_slice::<Line>(l).ok());
    let Some(Line::Header { preset, hash, .. }) = header else {
        return false;
    };
    Engine::new(&preset, RegistryLookup).is_ok_and(|engine| hash == hex(engine.snapshot_hash()))
}
