//! Fury — {3}{R}{R} — Creature — Elemental Incarnation
//! Oracle: Double strike
//! Oracle: When this creature enters, it deals 4 damage divided as you choose among any number of target creatures and/or planeswalkers.
//! Oracle: Evoke—Exile a red card from your hand.
//! Set: ECC #50 — Lorwyn Eclipsed Commander | Scryfall ID: 932d0fb9-382d-464a-9427-b91fa2398cdb | Oracle ID: fbf9f8c5-849f-45d5-8129-5fc683c21a04

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FURY,
    oracle_id = "fbf9f8c5-849f-45d5-8129-5fc683c21a04",
    scryfall_id = "932d0fb9-382d-464a-9427-b91fa2398cdb",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Fury",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::ELEMENTAL,
            subtypes::creature::INCARNATION
        ],
        power = Some(3),
        toughness = Some(3),
        // "Evoke—Exile a red card from your hand."
        alternative_costs = &[AlternativeCost {
            cost: cost!(ExileFromHand(&Filter::HasColor(ColorSet::from_slice(&[
                Color::Red
            ])))),
            condition: AltCondition::Always,
        }],
    ),],
    keywords = KeywordSet::DOUBLE_STRIKE,
    coverage = Coverage::Implemented,
    abilities = &[
        // "When this creature enters, it deals 4 damage divided as you choose
        // among any number of target creatures and/or planeswalkers." Any
        // number is at most four: each target is dealt at least 1
        // (CR 601.2d).
        triggered!(
            Trigger::ETB,
            &[Effect::DealDamageDivided { amount: 4 }],
            targets = Some(TargetReq::up_to(
                TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER),
                4
            )),
        ),
        triggered!(Trigger::EntersBattlefieldEvoked, &[Effect::SacrificeSelf]),
    ],
);
