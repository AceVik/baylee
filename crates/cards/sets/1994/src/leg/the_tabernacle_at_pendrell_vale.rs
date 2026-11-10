//! The Tabernacle at Pendrell Vale — (no cost) — Legendary Land
//! Oracle: All creatures have "At the beginning of your upkeep, destroy this creature unless you pay {1}."
//! Set: ME3 #212 — Masters Edition III | Scryfall ID: cd3f7f4e-cb25-4121-96a0-a4dc530420b9 | Oracle ID: 69b409b3-fa16-4c79-8b46-215a7036ed46
// IMPLEMENTED — one static grant to every creature (Modifier::GrantTriggered, layer 6):
// the controller's upkeep, pay {1} or the creature is destroyed.

use baylee_cards_dsl::prelude::*;

/// The ability the land hands to every creature: "At the beginning of your
/// upkeep, destroy this creature unless you pay {1}."
///
/// `You` is the ability's controller, which for a granted trigger is the
/// creature's controller and not the land's — the printed "your upkeep" and
/// the printed "you pay" are the same seat, and it is the seat the creature
/// sits in.
static TABERNACLE_TAX: &[Effect] = &[Effect::PlayerMayPayOr {
    player: PlayerRel::You,
    mana: Amount::Fixed(1),
    effect: &Effect::destroy(TargetSpec::ThisObject),
}];

card!(
    index = index::THE_TABERNACLE_AT_PENDRELL_VALE,
    oracle_id = "69b409b3-fa16-4c79-8b46-215a7036ed46",
    scryfall_id = "cd3f7f4e-cb25-4121-96a0-a4dc530420b9",
    faces = &[face!(
        name = "The Tabernacle at Pendrell Vale",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::CREATURE,
        Modifier::GrantTriggered {
            trigger: Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            effects: TABERNACLE_TAX,
            target: None,
        }
    )],
);
