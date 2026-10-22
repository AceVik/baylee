//! Veteran Explorer — {G} — Creature — Human Soldier Scout
//! Oracle: When this creature dies, each player may search their library for up to two basic land cards, put them onto the battlefield, then shuffle.
//! Set: 2XM #186 — Double Masters | Scryfall ID: 1a6c19cc-6469-4958-a547-1d0fe97c70e8 | Oracle ID: fa9677d2-1535-4c57-baa8-c0408e15da47
// PARTIAL — When this creature dies, each player searches their library for up to two basic land cards, puts them onto the battlefield, and shuffles; making each player's search optional before looking at their library is not yet modeled on EachPlayerSearchBeforeShuffle.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VETERAN_EXPLORER,
    oracle_id = "fa9677d2-1535-4c57-baa8-c0408e15da47",
    scryfall_id = "1a6c19cc-6469-4958-a547-1d0fe97c70e8",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "When this creature dies, each player searches their library for up to two basic land cards, puts them onto the battlefield, and shuffles; making each player's search optional before looking at their library is not yet modeled on EachPlayerSearchBeforeShuffle."
    ),
    faces = &[face!(
        name = "Veteran Explorer",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::SOLDIER,
            subtypes::creature::SCOUT
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[
            Effect::EachPlayerSearchBeforeShuffle {
                filter: &Filter::BASIC_LAND,
                find: &Find::BATTLEFIELD,
            },
            Effect::EachPlayerSearchBeforeShuffle {
                filter: &Filter::BASIC_LAND,
                find: &Find::BATTLEFIELD,
            },
            Effect::ShuffleLibrary {
                who: PlayerRel::EachPlayer,
            },
        ]
    ),],
);
