//! Shard Phoenix — {4}{R} — Creature — Phoenix
//! Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.)
//! Oracle: Sacrifice this creature: It deals 2 damage to each creature without flying.
//! Oracle: {R}{R}{R}: Return this card from your graveyard to your hand. Activate only during your upkeep.
//! Set: TPR #158 — Tempest Remastered | Scryfall ID: 611445e9-5dfc-4fc8-b936-02f81de65012 | Oracle ID: 3b629793-0efb-4d60-9d82-38b7b400abfd
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURE_WITHOUT_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

card!(
    index = index::SHARD_PHOENIX,
    oracle_id = "3b629793-0efb-4d60-9d82-38b7b400abfd",
    scryfall_id = "611445e9-5dfc-4fc8-b936-02f81de65012",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::FLYING,
    faces = &[face!(
        name = "Shard Phoenix",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHOENIX],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        activated!(
            cost!(SacrificeSelf),
            &[Effect::DealDamageEach {
                filter: &CREATURE_WITHOUT_FLYING,
                amount: Amount::Fixed(2),
            }],
        ),
        activated!(
            cost!("{R}{R}{R}"),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
            zone = ActivationZone::Graveyard,
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ])),
        ),
    ],
);
