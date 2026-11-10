//! Immersturm Skullcairn — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {1}{B}{R}{R}, {T}, Sacrifice this land: It deals 3 damage to target player. That player discards a card. Activate only as a sorcery.
//! Set: KHM #263 — Kaldheim | Scryfall ID: 12ed97de-736d-43d8-977b-308ac54f88f4 | Oracle ID: 354a7376-fb4b-424d-8964-93727302dccb
// IMPLEMENTED — enters tapped, {T} for {B}, and {1}{B}{R}{R}, {T}, sac deals 3 damage to target player and forces a discard at sorcery speed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IMMERSTURM_SKULLCAIRN,
    oracle_id = "354a7376-fb4b-424d-8964-93727302dccb",
    scryfall_id = "12ed97de-736d-43d8-977b-308ac54f88f4",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Immersturm Skullcairn",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            cost!("{1}{B}{R}{R}", TapSelf, SacrificeSelf),
            &[
                Effect::DealDamage {
                    amount: Amount::Fixed(3),
                    target: TargetSpec::Player(PlayerRel::Chosen),
                },
                Effect::DiscardForPlayers {
                    who: PlayerRel::Chosen,
                    count: 1,
                },
            ],
            target = Some(TargetSpec::AnyPlayer),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
