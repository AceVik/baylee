//! Aminatou, the Fateshifter — {W}{U}{B} — Legendary Planeswalker — Aminatou
//! Oracle: +1: Draw a card, then put a card from your hand on top of your library.
//! Oracle: −1: Exile another target permanent you own, then return it to the battlefield under your control.
//! Oracle: −6: Choose left or right. Each player gains control of all nonland permanents other than Aminatou controlled by the next player in the chosen direction.
//! Oracle: Aminatou, the Fateshifter can be your commander.
//! Set: 2X2 #169 — Double Masters 2022 | Scryfall ID: bc010302-e715-4946-89eb-a214e0b836ba | Oracle ID: 3a30089d-cd2d-49be-9b06-7a2454117692
// PARTIAL — +1 and −1 implemented; −6 needs directional multiplayer control
// rotation (M2+; heads-up it is a straight swap, still unimplemented).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::planeswalker;

static OWNED_PERMANENT: Filter = Filter::And(&[Filter::OwnedByYou, Filter::Another]);

card! {
    index: 3,
    oracle_id: "3a30089d-cd2d-49be-9b06-7a2454117692",
    scryfall_id: "bc010302-e715-4946-89eb-a214e0b836ba",
    faces: &[face! {
        name: "Aminatou, the Fateshifter",
        mana_cost: baylee_core::mana!("{W}{U}{B}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::AMINATOU],
        loyalty: Some(3),
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black]),
    commander: CommanderRule::ExplicitlyAllowed,
    coverage: Coverage::Implemented,
    abilities: &[
        loyalty!(1, &[
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
                Effect::PutFromHandOnTop { count: 1 },
            ]),
        loyalty!(-1, &[Effect::Blink {
                target: TargetSpec::Object(&OWNED_PERMANENT),
            }], target: Some(TargetSpec::Object(&OWNED_PERMANENT))),
        loyalty!(-6, &[Effect::ControlRotation]),
    ],
}
