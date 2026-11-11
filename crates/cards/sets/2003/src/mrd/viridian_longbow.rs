//! Viridian Longbow — {1} — Artifact — Equipment
//! Oracle: Equipped creature has "{T}: This creature deals 1 damage to any target."
//! Oracle: Equip {3} ({3}: Attach to target creature you control. Equip only as a sorcery.)
//! Set: AFC #221 — Forgotten Realms Commander | Scryfall ID: f4607634-6206-4c6c-b389-abcbfe969b65 | Oracle ID: 8b9119b0-9948-497f-aa92-2879d07bed1c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static VIRIDIAN_LONGBOW_PING: AbilityDef = activated!(
    Cost::TAP,
    &[Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::AnyTarget,
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
);

card!(
    index = index::VIRIDIAN_LONGBOW,
    oracle_id = "8b9119b0-9948-497f-aa92-2879d07bed1c",
    scryfall_id = "f4607634-6206-4c6c-b389-abcbfe969b65",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Viridian Longbow",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    abilities = &[
        equip!(cost!("{3}")),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&VIRIDIAN_LONGBOW_PING)
        ),
    ],
);
