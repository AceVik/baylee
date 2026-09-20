//! Gilt-Leaf Palace — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Elf card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {B} or {G}.
//! Set: LRW #268 — Lorwyn | Scryfall ID: cc4bbbb5-4218-4b2a-9ca2-de4d5f5dda19 | Oracle ID: 85573a3d-2993-491a-8f8d-bbdb844fa84e
// PARTIAL — the mana line only: `{T}: Add {B} or {G}`. The entry clause is
// not expressible, so the land always enters untapped.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GILT_LEAF_PALACE,
    oracle_id = "85573a3d-2993-491a-8f8d-bbdb844fa84e",
    scryfall_id = "cc4bbbb5-4218-4b2a-9ca2-de4d5f5dda19",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(name = "Gilt-Leaf Palace", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "as this land enters, you may reveal an Elf card from your hand; if you don't, it enters tapped",
    ),
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Black,
        ManaColor::Green,
    ])])],
);

// NOT SUPPORTED: "As this land enters, you may reveal an Elf card from your
// hand. If you don't, this land enters tapped." — no `EnterModifier` asks
// about a card in hand: the `TappedUnless` family reads the battlefield
// (`TappedUnlessCount`/`TappedUnlessAtMost`) or a player's life
// (`TappedUnlessOpponents`/`TappedUnlessSomeoneAtOrBelow`), and the reveal is
// an optional choice besides. The land therefore enters untapped every time
// and the reveal is dropped.
