//! Hidden Cataract — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {4}{U}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery. (Exile cards from the top of your library until you exile a nonland card with mana value 4 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)
//! Set: LCI #273 — The Lost Caverns of Ixalan | Scryfall ID: 69f317fc-f603-45b5-9208-545be4dcbf36 | Oracle ID: 927979d7-9b5c-4448-aef0-baf2907a89f1
// IMPLEMENTED — enters tapped and taps for {U}; the discover ability is not
// expressible and is left off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIDDEN_CATARACT,
    oracle_id = "927979d7-9b5c-4448-aef0-baf2907a89f1",
    scryfall_id = "69f317fc-f603-45b5-9208-545be4dcbf36",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "discover 4: no Effect exiles from the top of the library until a nonland card of mana value \
         <= N and lets its owner cast it for free (LookAtTopPick keeps a fixed count from a known \
         top, SearchLibrary searches by filter, and neither casts anything)"
    ),
    faces = &[face!(
        name = "Hidden Cataract",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        // NOT SUPPORTED: "{4}{U}, {T}, Sacrifice this land: Discover 4.
        // Activate only as a sorcery." — the cost is sayable, the effect is
        // not, and an activation that pays for nothing is worse than none.
    ],
);
