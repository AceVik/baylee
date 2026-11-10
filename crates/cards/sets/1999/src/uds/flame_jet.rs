//! Flame Jet — {1}{R} — Sorcery
//! Oracle: Flame Jet deals 3 damage to target player or planeswalker.
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: UDS #81 — Urza's Destiny | Scryfall ID: a511f9df-b53b-4fea-87cd-9f18f6833f92 | Oracle ID: 8a9d6a30-e2c2-4da5-ad3b-d64fb2b5543c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLAME_JET,
    oracle_id = "8a9d6a30-e2c2-4da5-ad3b-d64fb2b5543c",
    scryfall_id = "a511f9df-b53b-4fea-87cd-9f18f6833f92",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Flame Jet",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[
        spell!(
            &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
            }],
            targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
                &Filter::PLANESWALKER,
            ))),
        ),
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand
        ),
    ],
);
