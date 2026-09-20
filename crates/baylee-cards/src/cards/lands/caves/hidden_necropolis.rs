//! Hidden Necropolis — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {4}{B}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery. (Exile cards from the top of your library until you exile a nonland card with mana value 4 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)
//! Set: LCI #275 — The Lost Caverns of Ixalan | Scryfall ID: f67fd04f-05da-4418-97de-abeb7346cc69 | Oracle ID: f780ee53-62b0-4c32-b5b7-047651f48e5f
// PARTIAL — the tapland entry and {T}: Add {B}; the discover ability is
// dropped (no Effect can say "exile from the top until …").

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIDDEN_NECROPOLIS,
    oracle_id = "f780ee53-62b0-4c32-b5b7-047651f48e5f",
    scryfall_id = "f67fd04f-05da-4418-97de-abeb7346cc69",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Hidden Necropolis",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "Discover 4 has no DSL variant: nothing exiles cards from the top of a library \
         until a nonland card with mana value 4 or less, casts it without paying its \
         mana cost, and bottoms the rest in a random order"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        // NOT SUPPORTED: "{4}{B}, {T}, Sacrifice this land: Discover 4. Activate
        // only as a sorcery." — the DSL has no Effect for discover; LookAtTopPick
        // has a fixed count and no free cast, and no effect reads mana value while
        // walking a library.
    ],
);
