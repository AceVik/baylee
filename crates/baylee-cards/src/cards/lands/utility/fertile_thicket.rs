//! Fertile Thicket — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, you may look at the top five cards of your library. If you do, reveal up to one basic land card from among them, then put that card on top of your library and the rest on the bottom in any order.
//! Oracle: {T}: Add {G}.
//! Set: DDR #27 — Duel Decks: Nissa vs. Ob Nixilis | Scryfall ID: c30fb2f1-f0a6-430f-86e5-817b55da469e | Oracle ID: 7ba580c9-f933-43d9-b03d-a349faa6c641
// PARTIAL — the "enters tapped" replacement and `{T}: Add {G}` are built; the
// enters trigger has no effect in the DSL and is a NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FERTILE_THICKET,
    oracle_id = "7ba580c9-f933-43d9-b03d-a349faa6c641",
    scryfall_id = "c30fb2f1-f0a6-430f-86e5-817b55da469e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "the enters trigger: no Effect looks at the top five cards, reveals up to one \
         basic land card from among them, puts it on top of the library and the rest on \
         the bottom — LookAtTopPick sends the kept cards to hand and takes no filter, \
         and ReorderTopLibrary puts the whole pile back and bottoms nothing"
    ),
    faces = &[face!(
        name = "Fertile Thicket",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        // NOT SUPPORTED: "When this land enters, you may look at the top five cards of
        // your library. If you do, reveal up to one basic land card from among them,
        // then put that card on top of your library and the rest on the bottom in any
        // order." — no effect takes a filter over the top N, tops the chosen card and
        // bottoms the rest.
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
    ],
);
