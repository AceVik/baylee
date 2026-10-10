//! Serendib Djinn — {2}{U}{U} — Creature — Djinn
//! Oracle: Flying
//! Oracle: At the beginning of your upkeep, sacrifice a land. If you sacrifice an Island this way, this creature deals 3 damage to you.
//! Oracle: When you control no lands, sacrifice this creature.
//! Set: ME4 #61 — Masters Edition IV | Scryfall ID: ed1409f0-f17b-4f5c-9cf7-8ed18143b7b8 | Oracle ID: 683e7135-de54-49c8-a978-4f84628a6a91
// IMPLEMENTED — flying, the upkeep land sacrifice with its Island damage
// (Effect::SacrificeOneThen), and the no-lands sacrifice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);

card!(
    index = index::SERENDIB_DJINN,
    oracle_id = "683e7135-de54-49c8-a978-4f84628a6a91",
    scryfall_id = "ed1409f0-f17b-4f5c-9cf7-8ed18143b7b8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    faces = &[face!(
        name = "Serendib Djinn",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(5),
        toughness = Some(6),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::SacrificeOneThen {
                filter: &Filter::LAND,
                if_it_was: &ISLAND,
                then: &[Effect::DealDamage {
                    amount: Amount::Fixed(3),
                    target: TargetSpec::Player(PlayerRel::You),
                }],
            }]
        ),
        triggered!(
            Trigger::State(&Condition::ControlCountAtMost(&Filter::LAND, 0)),
            &[Effect::SacrificeSelf]
        ),
    ],
);
