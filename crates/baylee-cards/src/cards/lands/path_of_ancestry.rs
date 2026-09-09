//! Path of Ancestry — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add one mana of any color in your commander's color identity. When that mana is spent to cast a creature spell that shares a creature type with your commander, scry 1. (Look at the top card of your library. You may put that card on the bottom.)
//! Set: MBC #80 — Mystery Booster Commander Edition | Scryfall ID: b1aaa7b0-1cac-4a92-b880-7ef1ac00618f | Oracle ID: b473e293-59e3-4e04-acf2-622604aeb25f
// IMPLEMENTED — enters tapped + commander-identity mana with the scry
// rider: restricted mana that scries 1 when spent on a creature spell
// sharing a type with your commander.
// NOTE: the scry trigger queues via the synthetic-ability path (stacked
// as an ability).

static COMMANDER_TYPE_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::SharesSubtypeWithCommander]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 111,
    oracle_id: "b473e293-59e3-4e04-acf2-622604aeb25f",
    scryfall_id: "b1aaa7b0-1cac-4a92-b880-7ef1ac00618f",
    faces: &[face! {
        name: "Path of Ancestry",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    }],
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_commander_identity().restricted(
            &COMMANDER_TYPE_CREATURE_SPELL,
            SpendRider::Scry(1),
        )])],
}
