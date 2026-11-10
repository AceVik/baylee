//! Ashes to Ashes — {1}{B}{B} — Sorcery
//! Oracle: Exile two target nonartifact creatures. Ashes to Ashes deals 5 damage to you.
//! Set: ME3 #58 — Masters Edition III | Scryfall ID: 9e61209b-0f56-4bab-9ada-700f91b18dec | Oracle ID: 944a52d9-bb14-43c5-8d05-2afaf023dc9f

use baylee_cards_dsl::prelude::*;

const NONARTIFACT_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::LacksType(TypeSet::ARTIFACT)]);

card!(
    index = index::ASHES_TO_ASHES,
    oracle_id = "944a52d9-bb14-43c5-8d05-2afaf023dc9f",
    scryfall_id = "9e61209b-0f56-4bab-9ada-700f91b18dec",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ashes to Ashes",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::Exile {
                target: TargetSpec::Object(&NONARTIFACT_CREATURE),
            },
            Effect::DealDamage {
                amount: Amount::Fixed(5),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ],
        targets = Some(TargetReq::exactly(
            TargetSpec::Object(&NONARTIFACT_CREATURE),
            2,
        )),
    )],
);
