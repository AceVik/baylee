//! Abyssal Specter — {2}{B}{B} — Creature — Specter
//! Oracle: Flying
//! Oracle: Whenever this creature deals damage to a player, that player discards a card.
//! Set: MM3 #59 — Modern Masters 2017 | Scryfall ID: 69a41bec-7fb0-4060-918a-38cbbb329543 | Oracle ID: 51833cff-6519-4806-8d91-0040e9a02189

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ABYSSAL_SPECTER,
    oracle_id = "51833cff-6519-4806-8d91-0040e9a02189",
    scryfall_id = "69a41bec-7fb0-4060-918a-38cbbb329543",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::FLYING,
    faces = &[face!(
        name = "Abyssal Specter",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPECTER],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: PlayerRel::EachPlayer,
        },
        &[Effect::DiscardForPlayers {
            who: PlayerRel::DamagedPlayer,
            count: 1,
        }]
    )],
);
