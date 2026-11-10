//! Phantasmal Mount — {1}{U} — Creature — Illusion Horse
//! Oracle: Flying
//! Oracle: {T}: Target creature you control with toughness 2 or less gets +1/+1 and gains flying until end of turn. When this creature leaves the battlefield this turn, sacrifice that creature. When the creature leaves the battlefield this turn, sacrifice this creature.
//! Set: ME2 #59 — Masters Edition II | Scryfall ID: f4d44f96-2932-4ce9-b531-d9b7cfac391b | Oracle ID: 8c7723a5-04c7-4241-b0ba-6a821449ca43
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_SMALL_CREATURE: Filter =
    Filter::And(&[Filter::YOUR_CREATURE, Filter::ToughnessAtMost(2)]);

card!(
    index = index::PHANTASMAL_MOUNT,
    oracle_id = "8c7723a5-04c7-4241-b0ba-6a821449ca43",
    scryfall_id = "f4d44f96-2932-4ce9-b531-d9b7cfac391b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Phantasmal Mount",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION, subtypes::creature::HORSE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::FLYING,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::WhenSourceLeavesBattlefieldThisTurn {
                effects: &[Effect::SacrificeObject {
                    target: TargetSpec::EventObject,
                }],
            },
            Effect::WhenTargetLeavesBattlefieldThisTurn {
                effects: &[Effect::SacrificeSelf],
            },
        ],
        target = Some(TargetSpec::Object(&YOUR_SMALL_CREATURE))
    )],
);
