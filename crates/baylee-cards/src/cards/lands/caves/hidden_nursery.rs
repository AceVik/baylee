//! Hidden Nursery — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {4}{G}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery. (Exile cards from the top of your library until you exile a nonland card with mana value 4 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)
//! Set: LCI #276 — The Lost Caverns of Ixalan | Scryfall ID: a942939a-c06e-4b90-a404-ae5acfffcff9 | Oracle ID: 1a26e2d6-6bfc-4cdc-9bd6-8b37a9be2961
// PARTIAL — enters tapped + {T}: Add {G}; discover 4 has no effect in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIDDEN_NURSERY,
    oracle_id = "1a26e2d6-6bfc-4cdc-9bd6-8b37a9be2961",
    scryfall_id = "a942939a-c06e-4b90-a404-ae5acfffcff9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial("no Effect performs a discover"),
    faces = &[face!(
        name = "Hidden Nursery",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        // NOT SUPPORTED: {4}{G}, {T}, Sacrifice this land: Discover 4. Activate
        // only as a sorcery. — no Effect in the DSL exiles cards from the top
        // of a library until a nonland card of mana value 4 or less is found,
        // nor casts it for free or puts it into hand with the rest on the
        // bottom in a random order.
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
    ],
);
