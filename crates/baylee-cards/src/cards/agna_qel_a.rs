//! Agna Qel'a — (no cost) — Land
//! Oracle: This land enters tapped unless you control a basic land.
//! Oracle: {T}: Add {U}.
//! Oracle: {2}{U}, {T}: Draw a card, then discard a card.
//! Set: TLA #264 — Avatar: The Last Airbender | Scryfall ID: 6b885829-a323-4f7d-87c9-aa4615dcbe5c | Oracle ID: 22d0a848-2126-48f0-9050-38daaf93b1d0
// IMPLEMENTED — checkland variant + loot activation.

use baylee_cards_dsl::prelude::*;

static BASIC_LAND_YOU: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LAND,
    Filter::HasSupertype(SupertypeSet::BASIC),
]);

card! {
    index: 212,
    oracle_id: "22d0a848-2126-48f0-9050-38daaf93b1d0",
    scryfall_id: "6b885829-a323-4f7d-87c9-aa4615dcbe5c",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
        face! {
            name: "Agna Qel'a",
            types: TypeSet::LAND,
            enter_modifiers: &[EnterModifier::TappedUnless(&BASIC_LAND_YOU)],
        },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{2}{U}"),
                parts: &[CostPart::TapSelf],
            },
            &[
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
                Effect::DiscardForPlayers {
                    who: PlayerRel::You,
                    count: 1,
                },
            ]
        ),
    ],
}

// Engine-level test: enters tapped unless controller has a basic land; taps for
// {U}; {2}{U},{T} draws a card, then prompts controller to discard a card.
