//! Hidden Courtyard — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Oracle: {4}{W}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery. (Exile cards from the top of your library until you exile a nonland card with mana value 4 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)
//! Set: LCI #274 — The Lost Caverns of Ixalan | Scryfall ID: b8685d46-99fc-44b3-be95-707a4b7b8327 | Oracle ID: e19d5071-4ea1-4883-b067-a21e553f96e0
// PARTIAL — enters tapped and `{T}: Add {W}`; the discover ability is dropped,
// because no effect in the DSL exiles from the top of a library until a
// nonland card of a given mana value shows up.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIDDEN_COURTYARD,
    oracle_id = "e19d5071-4ea1-4883-b067-a21e553f96e0",
    scryfall_id = "b8685d46-99fc-44b3-be95-707a4b7b8327",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Hidden Courtyard",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "Discover 4 is not expressible: the DSL has no effect that exiles \
         from the top of a library until a nonland card of a given mana \
         value is exiled, and none that casts the exiled card for free",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        // NOT SUPPORTED: "{4}{W}, {T}, Sacrifice this land: Discover 4.
        // Activate only as a sorcery." — the cost and the sorcery-speed
        // timing are sayable, the discover effect is not, so the whole
        // ability is off the card rather than offered as a cost that pays
        // for nothing.
    ],
);
