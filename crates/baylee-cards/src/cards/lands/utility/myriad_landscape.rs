//! Myriad Landscape — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Search your library for up to two basic land cards that share a land type, put them onto the battlefield tapped, then shuffle.
//! Set: EOC #169 — Edge of Eternities Commander | Scryfall ID: a0e2098f-1d94-491a-a7e9-a45a9f69e3a8 | Oracle ID: 2549bc57-9ffb-4053-9f10-f2a5f792b845
// PARTIAL — enters tapped and {T}: Add {C}; the sacrifice-tutor is left off
// because no Filter can say "share a land type".

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MYRIAD_LANDSCAPE,
    oracle_id = "2549bc57-9ffb-4053-9f10-f2a5f792b845",
    scryfall_id = "a0e2098f-1d94-491a-a7e9-a45a9f69e3a8",
    coverage = Coverage::Partial(
        "the third ability finds two basic land cards that share a land type — a relation between the two finds, which no Filter can express",
    ),
    faces = &[face!(
        name = "Myriad Landscape",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: {2}, {T}, Sacrifice this land: Search your library
        // for up to two basic land cards that share a land type, put them
        // onto the battlefield tapped, then shuffle. — `SearchLibrary` takes
        // a `Filter` and a list of `Find`s, and the printed restriction is
        // about the *pair*: the second card must share a land type with the
        // first, which `Filter::BASIC_LAND` cannot say. Written as two
        // `BASIC_LAND` finds the ability would let a player take a Forest
        // and a Plains, so the ability is left off rather than widened.
    ],
);
