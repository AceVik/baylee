//! Magosi, the Waterveil — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {U}, {T}: Put an eon counter on this land. Skip your next turn.
//! Oracle: {T}, Remove an eon counter from this land and return it to its owner's hand: Take an extra turn after this one.
//! Set: ZEN #218 — Zendikar | Scryfall ID: 3c84cf70-4164-47ea-8da1-c0ca1ac132e1 | Oracle ID: 4bdffa67-e6b3-4588-b76e-c11db6f043ca

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MAGOSI_THE_WATERVEIL,
    oracle_id = "4bdffa67-e6b3-4588-b76e-c11db6f043ca",
    scryfall_id = "3c84cf70-4164-47ea-8da1-c0ca1ac132e1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Magosi, the Waterveil",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial("Batch 49 package validation pending"),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        activated!(
            cost!("{U}", TapSelf),
            &[
                Effect::AddCounter { kind: counters::EON, amount: Amount::Fixed(1) },
                Effect::SkipNextTurn,
            ]
        ),
        activated!(
            cost!(TapSelf, RemoveCounterSelf { kind: counters::EON, n: 1 }, ReturnSelfToHand),
            &[Effect::TakeExtraTurn]
        ),
    ],
);
