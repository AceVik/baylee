//! Goblin Ski Patrol — {1}{R} — Creature — Goblin
//! Oracle: {1}{R}: This creature gets +2/+0 and gains flying. Its controller sacrifices it at the beginning of the next end step. Activate only once and only if you control a snow Mountain.
//! Set: ME2 #128 — Masters Edition II | Scryfall ID: 6827594e-3912-44f2-bd84-bf7ef69276de | Oracle ID: e98396d0-6952-4755-992f-813f565b9189
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SNOW_MOUNTAIN: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
    Filter::HasSupertype(SupertypeSet::SNOW),
]);

card!(
    index = index::GOBLIN_SKI_PATROL,
    oracle_id = "e98396d0-6952-4755-992f-813f565b9189",
    scryfall_id = "6827594e-3912-44f2-bd84-bf7ef69276de",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Ski Patrol",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{R}"),
        &[
            Effect::ModifySource {
                modifier: Modifier::ModifyPT(2, 0),
                duration: Duration::Indefinitely,
            },
            Effect::ModifySource {
                modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                duration: Duration::Indefinitely,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::SacrificeEvent],
            },
        ],
        limit = ActivationLimit::Exhaust,
        condition = Some(Condition::ControlCount(&SNOW_MOUNTAIN, 1))
    )],
);
