//! Phyrexian War Beast — {3} — Artifact Creature — Phyrexian Beast
//! Oracle: When this creature leaves the battlefield, sacrifice a land and this creature deals 1 damage to you.
//! Set: ME1 #162 — Masters Edition | Scryfall ID: 6c7576e2-1a95-453f-aab5-b08e21f28ba4 | Oracle ID: 0c93c418-e10b-4a6d-bddb-3b786bb6a928
// IMPLEMENTED — leaves-the-battlefield trigger sacrificing a land and dealing 1 damage to you.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHYREXIAN_WAR_BEAST,
    oracle_id = "0c93c418-e10b-4a6d-bddb-3b786bb6a928",
    scryfall_id = "6c7576e2-1a95-453f-aab5-b08e21f28ba4",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Phyrexian War Beast",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::LeavesBattlefield(&Filter::This),
        &[
            Effect::SacrificeFilter {
                who: PlayerRel::You,
                filter: &Filter::LAND,
            },
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ],
    )],
);
