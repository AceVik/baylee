//! Which ending a seat takes where more than one "instead" wants a resolved
//! spell (CR 616.1e).
//!
//! The card back in hand keeps it, at its full price each time. Rebound is one
//! free cast and then the card is gone. So a seat that can pay for the spell
//! again takes the card, and one that cannot takes the cast it will not have
//! to pay for. A reaction goes home whatever the lands say: the next upkeep has
//! nothing on the stack for a counterspell to answer. The free creature of a
//! rebound beats paying for it off an adventure, and an exile that gives
//! nothing back comes last.
//!
//! Choosing for a player one controls (CR 723.5) reads the same list from the
//! other end.

use baylee_cards_dsl::AbilityDef;
use baylee_client_core::decision::resource_player;
use baylee_core::ids::ObjectId;
use baylee_core::types::TypeSet;
use baylee_engine::choice::{CastModeDesc, CastModeKind, SpellEnding};
use baylee_view::PlayerView;

/// Whether the card is worth more in hand than one free cast of it.
fn keeps_the_card(view: &PlayerView, object: ObjectId) -> bool {
    let Some(rules) = view.object(object).and_then(|seen| seen.rules) else {
        return true;
    };
    let Some(def) = baylee_cards::by_index(rules.card) else {
        return true;
    };
    let index = usize::from(rules.face);
    let reacts = def.abilities_for_face(index).iter().any(|ability| {
        matches!(ability, AbilityDef::Spell { effects, .. }
            if effects.iter().any(crate::controlled::counters))
    });
    let Some(face) = def.faces.get(index).filter(|_| !reacts) else {
        return true;
    };
    let again = face.mana_cost.with_x(0).cmc() + face.buyback.map_or(0, |cost| cost.mana.cmc());
    let lands = view
        .battlefield_of(resource_player(view))
        .filter(|land| land.types.contains(TypeSet::LAND))
        .count();
    u32::try_from(lands).is_ok_and(|lands| lands >= again)
}

/// Each ending in `options` with its position and how good it is for the
/// spell's controller, the lower the better.
fn ranked(view: &PlayerView, object: ObjectId, options: &[CastModeDesc]) -> Vec<(u8, usize)> {
    let hand_first = keeps_the_card(view, object);
    options
        .iter()
        .enumerate()
        .filter_map(|(at, option)| match option.kind {
            CastModeKind::SpellEnding(ending) => Some((
                match ending {
                    SpellEnding::Hand if hand_first => 0,
                    SpellEnding::Rebound => 1,
                    SpellEnding::Hand => 2,
                    SpellEnding::Adventure => 3,
                    SpellEnding::Exile => 4,
                },
                at,
            )),
            _ => None,
        })
        .collect()
}

/// The position of the ending a seat takes for its own spell; `None` for a
/// question that is not one of these.
pub(crate) fn own(view: &PlayerView, object: ObjectId, options: &[CastModeDesc]) -> Option<usize> {
    ranked(view, object, options)
        .into_iter()
        .min()
        .map(|(_, at)| at)
}

/// The position of the ending worst for the player `theirs` shows, whose
/// spell it is; `None` for a question that is not one of these.
pub(crate) fn against(
    theirs: &PlayerView,
    object: ObjectId,
    options: &[CastModeDesc],
) -> Option<usize> {
    ranked(theirs, object, options)
        .into_iter()
        .max()
        .map(|(_, at)| at)
}
