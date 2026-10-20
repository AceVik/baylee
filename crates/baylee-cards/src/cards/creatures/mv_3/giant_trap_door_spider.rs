//! Giant Trap Door Spider — {1}{R}{G} — Creature — Spider
//! Oracle: {1}{R}{G}, {T}: Exile this creature and target creature without flying that's attacking you.
//! Set: ME2 #195 — Masters Edition II | Scryfall ID: b72846a2-a015-486d-804b-c2a2df045444 | Oracle ID: 2c87565b-3902-4b11-b894-6922f66008ba
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ATTACKING_YOU_NO_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
    Filter::AttackingYou,
]);

card!(
    index = index::GIANT_TRAP_DOOR_SPIDER,
    oracle_id = "2c87565b-3902-4b11-b894-6922f66008ba",
    scryfall_id = "b72846a2-a015-486d-804b-c2a2df045444",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Giant Trap Door Spider",
        mana_cost = mana!("{1}{R}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIDER],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{1}{R}{G}", TapSelf),
        &[
            Effect::ExileSource,
            Effect::exile(TargetSpec::Object(&ATTACKING_YOU_NO_FLYING)),
        ],
        target = Some(TargetSpec::Object(&ATTACKING_YOU_NO_FLYING))
    )],
);
