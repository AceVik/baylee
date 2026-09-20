//! City of Ass — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add one and one-half mana of any one color.
//! Set: UNH #134 — Unhinged | Scryfall ID: 726fe2e7-bd1b-4c11-a098-6c8abfbdf06e | Oracle ID: e043a795-6936-4d7e-9a77-e0175a27c8f5
// PARTIAL — enters tapped (EnterModifier::Tapped); the {T} ability prints a
// fractional amount and comes off the card, see NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CITY_OF_ASS,
    oracle_id = "e043a795-6936-4d7e-9a77-e0175a27c8f5",
    scryfall_id = "726fe2e7-bd1b-4c11-a098-6c8abfbdf06e",
    faces = &[face!(
        name = "City of Ass",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the {T} ability adds one and one-half mana of any one color, and \
         the DSL's mana amounts are whole numbers",
    ),
);

// NOT SUPPORTED: {T}: Add one and one-half mana of any one color. `Amount`
// holds whole numbers only (`Amount::Fixed(u32)`) and no `ManaSource` carries
// a fraction, so half a mana is not a value `Effect::mana` / `AddMana` can
// express. The ability is off the card rather than silently paid out as one
// mana or two, which would be a different card.
