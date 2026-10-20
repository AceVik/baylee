//! Gangrenous Zombies — {1}{B}{B} — Creature — Zombie
//! Oracle: {T}, Sacrifice this creature: This creature deals 1 damage to each creature and each player. If you control a snow Swamp, this creature deals 2 damage to each creature and each player instead.
//! Set: ME2 #92 — Masters Edition II | Scryfall ID: 5b49b583-ed1b-44ee-8f50-be5ae3527399 | Oracle ID: d0d844f2-6cf4-49f2-95db-64e932a15b17
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SNOW_SWAMP: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::SWAMP),
    Filter::HasSupertype(SupertypeSet::SNOW),
]);

card!(
    index = index::GANGRENOUS_ZOMBIES,
    oracle_id = "d0d844f2-6cf4-49f2-95db-64e932a15b17",
    scryfall_id = "5b49b583-ed1b-44ee-8f50-be5ae3527399",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gangrenous Zombies",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[Effect::IfCondition {
            condition: Condition::ControlCount(&SNOW_SWAMP, 1),
            then: &[
                Effect::DealDamageEach {
                    amount: Amount::Fixed(2),
                    filter: &Filter::CREATURE,
                },
                Effect::DealDamage {
                    target: TargetSpec::Player(PlayerRel::EachPlayer),
                    amount: Amount::Fixed(2),
                },
            ],
            otherwise: &[
                Effect::DealDamageEach {
                    amount: Amount::Fixed(1),
                    filter: &Filter::CREATURE,
                },
                Effect::DealDamage {
                    target: TargetSpec::Player(PlayerRel::EachPlayer),
                    amount: Amount::Fixed(1),
                },
            ],
        }]
    )],
);
