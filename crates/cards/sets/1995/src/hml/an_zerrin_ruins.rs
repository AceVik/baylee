//! An-Zerrin Ruins — {2}{R}{R} — Enchantment
//! Oracle: As this enchantment enters, choose a creature type.
//! Oracle: Creatures of the chosen type don't untap during their controllers' untap steps.
//! Set: ME2 #117 — Masters Edition II | Scryfall ID: 6c738baa-61c4-4b3c-9311-1e69bde0ed8a | Oracle ID: f805b100-49b8-4d23-a768-e8de8bb0daa3
// IMPLEMENTED — chooses a creature type on entry and keeps creatures of that type from untapping during their controllers' untap steps.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AN_ZERRIN_RUINS,
    oracle_id = "f805b100-49b8-4d23-a768-e8de8bb0daa3",
    scryfall_id = "6c738baa-61c4-4b3c-9311-1e69bde0ed8a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "An-Zerrin Ruins",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseSubtype],
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]),
        Modifier::DoesNotUntap
    )],
);
