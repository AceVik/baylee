//! Steely Resolve — {1}{G} — Enchantment
//! Oracle: As this enchantment enters, choose a creature type.
//! Oracle: Creatures of the chosen type have shroud. (They can't be the targets of spells or abilities.)
//! Set: ONS #286 — Onslaught | Scryfall ID: b88c530a-abc3-4cc4-8a48-5b76e1504a3c | Oracle ID: 48c127f0-2857-4c36-97bc-1291b6fe4a82
// IMPLEMENTED — the type is chosen as it enters (EnterModifier::ChooseSubtype)
// and a layer-6 static grants shroud to every creature of that type
// (Filter::MatchesChosenTypeOfSource).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STEELY_RESOLVE,
    oracle_id = "48c127f0-2857-4c36-97bc-1291b6fe4a82",
    scryfall_id = "b88c530a-abc3-4cc4-8a48-5b76e1504a3c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Steely Resolve",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseSubtype],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]),
        Modifier::AddKeyword(KeywordSet::SHROUD)
    )],
);
