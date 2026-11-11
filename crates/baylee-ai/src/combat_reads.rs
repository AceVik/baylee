//! What a creature's printed abilities change about a block
//! (`AIProfile::BLOCK_READS`), read from the public face of the permanent.
//!
//! The combat search prices a creature by its mana value, power and
//! toughness, and lets lethal damage kill it. Two printed abilities make both
//! wrong in a way a block hinges on: a creature that regenerates itself, with
//! the mana for it open, does not die of the damage (CR 701.19a), so a block
//! that "trades" with it only loses the blocker; and a creature whose own
//! death does something for its controller (`Trigger::Dies` of itself: a
//! card drawn, a token made, a return) is cheaper to lose than its body says,
//! for whichever side it is on.

use baylee_cards_dsl::{AbilityDef, Effect, Filter, TargetSpec, Trigger};
use baylee_core::types::TypeSet;
use baylee_view::{ObjectStatus, PlayerView, PublicObject};

/// The abilities printed on the face `o` shows, or none where the view does
/// not say which card it is.
fn printed(o: &PublicObject) -> &'static [AbilityDef] {
    o.rules
        .and_then(|rules| {
            baylee_cards::by_index(rules.card)
                .map(|def| def.abilities_for_face(usize::from(rules.face)))
        })
        .unwrap_or(&[])
}

/// Whether `o` can regenerate itself now: it prints an ability that
/// regenerates it for mana alone (no tap, no other cost), and its controller
/// has at least that much mana in untapped lands. Colours are not checked.
pub(crate) fn regenerates(view: &PlayerView, o: &PublicObject) -> bool {
    let cheapest = printed(o)
        .iter()
        .filter_map(|ability| match ability {
            AbilityDef::Activated { cost, effects, .. }
                if cost.parts.is_empty()
                    && effects.iter().any(|effect| {
                        matches!(
                            effect,
                            Effect::Regenerate {
                                target: TargetSpec::ThisObject
                            }
                        )
                    }) =>
            {
                Some(cost.mana.cmc())
            }
            _ => None,
        })
        .min();
    let Some(cheapest) = cheapest else {
        return false;
    };
    let open = view
        .battlefield_of(o.controller)
        .filter(|land| land.types.contains(TypeSet::LAND))
        .filter(|land| !land.status.contains(ObjectStatus::TAPPED))
        .count();
    u32::try_from(open).is_ok_and(|open| open >= cheapest)
}

/// Whether `o`'s own death triggers something of its card (`When this
/// creature dies`).
pub(crate) fn dies_for_value(o: &PublicObject) -> bool {
    printed(o).iter().any(|ability| {
        matches!(
            ability,
            AbilityDef::Triggered {
                trigger: Trigger::Dies(Filter::This) | Trigger::DiesNotSacrificed(Filter::This),
                ..
            }
        )
    })
}
