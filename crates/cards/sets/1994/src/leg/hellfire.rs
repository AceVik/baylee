//! Hellfire — {2}{B}{B}{B} — Sorcery
//! Oracle: Destroy all nonblack creatures. Hellfire deals X plus 3 damage to you, where X is the number of creatures that died this way.
//! Set: ME3 #70 — Masters Edition III | Scryfall ID: e66002e6-f722-429c-94a0-f571d9110fbd | Oracle ID: e7b1975d-9574-4333-8530-33167948078a
// IMPLEMENTED — destroys all nonblack creatures, then deals X plus 3 damage to
// you, X being the creatures that died this way.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HELLFIRE,
    oracle_id = "e7b1975d-9574-4333-8530-33167948078a",
    scryfall_id = "e66002e6-f722-429c-94a0-f571d9110fbd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hellfire",
        mana_cost = mana!("{2}{B}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::destroy_all(&Filter::And(&[
            Filter::CREATURE,
            Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
        ])),
        // "Hellfire deals X plus 3 damage to you, where X is the number of
        // creatures that died this way."
        Effect::DealDamage {
            amount: Amount::Plus {
                base: &Amount::ThisWay(ThisWay::Died(&Filter::CREATURE)),
                offset: 3,
            },
            target: TargetSpec::Player(PlayerRel::You),
        },
    ])],
);
