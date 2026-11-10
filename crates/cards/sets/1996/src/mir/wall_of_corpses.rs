//! Wall of Corpses — {1}{B} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {B}, Sacrifice this creature: Destroy target creature this creature is blocking.
//! Set: MIR #151 — Mirage | Scryfall ID: dde6d3d1-75db-445f-9f17-632ee0292211 | Oracle ID: 16a7895a-ca23-4112-a91f-4b1528e75cea
// IMPLEMENTED — defender and {B}, Sacrifice: destroy target creature this creature is blocking.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURE_BLOCKED_BY_SOURCE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::BlockedBySource]);

card!(
    index = index::WALL_OF_CORPSES,
    oracle_id = "16a7895a-ca23-4112-a91f-4b1528e75cea",
    scryfall_id = "dde6d3d1-75db-445f-9f17-632ee0292211",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wall of Corpses",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(2),
        keywords = KeywordSet::DEFENDER,
    ),],
    abilities = &[activated!(
        cost!("{B}", SacrificeSelf),
        &[Effect::destroy(TargetSpec::Object(
            &CREATURE_BLOCKED_BY_SOURCE
        ))],
        target = Some(TargetSpec::Object(&CREATURE_BLOCKED_BY_SOURCE)),
    )],
);
