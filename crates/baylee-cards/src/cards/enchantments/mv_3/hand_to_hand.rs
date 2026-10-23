//! Hand to Hand — {2}{R} — Enchantment
//! Oracle: During combat, players can't cast instant spells or activate abilities that aren't mana abilities.
//! Set: TMP #180 — Tempest | Scryfall ID: af4cb86a-db01-4d9a-99e9-bb50ce23507f | Oracle ID: bf45f579-539c-4ffe-89df-4896a394158f
// PARTIAL — Prevents all players from casting instant spells during combat; preventing players from activating non-mana abilities during combat while still allowing mana abilities is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HAND_TO_HAND,
    oracle_id = "bf45f579-539c-4ffe-89df-4896a394158f",
    scryfall_id = "af4cb86a-db01-4d9a-99e9-bb50ce23507f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Prevents all players from casting instant spells during combat; preventing players from activating non-mana abilities during combat while still allowing mana abilities is not yet modeled."
    ),
    faces = &[face!(
        name = "Hand to Hand",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::PlayersCantPlay(&Filter::HasType(TypeSet::INSTANT)),
        condition = Some(Condition::DuringCombat),
    )],
);
