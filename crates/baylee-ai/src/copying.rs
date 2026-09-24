//! Choosing what a clone enters as a copy of (CR 707.2).
//!
//! The engine asks it as a target question with no effect behind it —
//! copying is an ability, not an effect — so read through
//! [`crate::tactics::meaning`] it meant nothing, and the fallback's first
//! rule, an opponent's permanent before anything else, decided it: Phyrexian
//! Metamorph copied the opponent's Llanowar Elves over its own controller's
//! Serra Angel (#227). [`DecisionContext::copying`] now says which question
//! it is, and the answer is the other way round from a removal spell's: the
//! copy is this seat's whoever controls the original, so what counts is only
//! what is worth having twice.

use baylee_cards_dsl::CopyMod;
use baylee_core::ids::ObjectId;
use baylee_core::types::SupertypeSet;
use baylee_engine::choice::PlayerAction;
use baylee_engine::engine::DecisionContext;
use baylee_view::{PlayerView, PublicObject};

use crate::tactics::material;

/// What a clone's question should name, or `None` when the question is not
/// a clone's.
///
/// Always one candidate, never none: a clone that copies nothing enters as
/// what it prints, which for most of the pool is a 0/0.
pub(crate) fn copy_target(
    view: &PlayerView,
    objects: &[ObjectId],
    context: &DecisionContext<'_>,
) -> Option<PlayerAction> {
    let mods = context.copying?;
    let best = objects.iter().copied().max_by_key(|&id| {
        (
            view.object(id).map_or(i64::MIN, |o| worth(view, o, mods)),
            std::cmp::Reverse(id),
        )
    })?;
    Some(PlayerAction::ChooseTargets {
        objects: vec![best],
        players: vec![],
    })
}

/// What a copy of `original` is worth to the seat that makes it.
///
/// Its copiable values are what the copy gets (CR 707.2): the printed size,
/// not a pump or the counters on the original, and not its commander status
/// either. A copy of a legendary permanent this seat already controls puts
/// two of one name under one controller, and the legend rule keeps one
/// (CR 704.5j) — unless the copy stops being legendary, as Spark Double's
/// does. An opponent's legend is no such problem: the rule is per player.
fn worth(view: &PlayerView, original: &PublicObject, mods: &[CopyMod]) -> i64 {
    let copiable = PublicObject {
        power: original.base_power,
        toughness: original.base_toughness,
        commander: false,
        ..original.clone()
    };
    let stays_legendary = original.supertypes.contains(SupertypeSet::LEGENDARY)
        && !mods.iter().any(
            |m| matches!(m, CopyMod::RemoveSupertype(s) if s.contains(SupertypeSet::LEGENDARY)),
        );
    if stays_legendary
        && view
            .battlefield_of(view.seat)
            .any(|mine| mine.name == original.name)
    {
        return 0;
    }
    material(&copiable)
}
