//! Skyshroud Elite — {G} — Creature — Elf
//! Oracle: This creature gets +1/+2 as long as an opponent controls a nonbasic land.
//! Set: EXO #123 — Exodus | Scryfall ID: f6a496a4-1b4c-4c5d-99e5-ec40601c759d | Oracle ID: bd9f2c84-0589-4ffe-9bff-ea74be4f77a4
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OPPONENT_NONBASIC_LAND: Filter = Filter::And(&[
    Filter::HasType(TypeSet::LAND),
    Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC)),
    Filter::ControlledByOpponent,
]);

card!(
    index = index::SKYSHROUD_ELITE,
    oracle_id = "bd9f2c84-0589-4ffe-9bff-ea74be4f77a4",
    scryfall_id = "f6a496a4-1b4c-4c5d-99e5-ec40601c759d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Skyshroud Elite",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[AbilityDef::Static(StaticAbility {
        source_zone: ZoneRef::Battlefield,
        layer: Layer::PtModify,
        filter: Filter::This,
        modifier: Modifier::ModifyPT(1, 2),
        condition: Some(Condition::OpponentControlCount(&OPPONENT_NONBASIC_LAND, 1)),
    })],
);
