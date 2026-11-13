//! Ninja's Kunai — {1} — Artifact — Equipment
//! Oracle: Equipped creature has "{1}, {T}, Sacrifice Ninja's Kunai: Ninja's Kunai deals 3 damage to any target."
//! Oracle: Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
//! Set: NEO #252 — Kamigawa: Neon Dynasty | Scryfall ID: af6ae567-c77a-416a-b609-4df3d0dd2e18 | Oracle ID: fa5aefaf-432f-41d0-8e54-0e8b7bc7ba98
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NINJA_S_KUNAI_SHOT: AbilityDef = activated!(
    cost!("{1}", TapSelf, SacrificeGrantor),
    &[Effect::GrantorDealsDamage {
        amount: Amount::Fixed(3),
        target: TargetSpec::AnyTarget,
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
);

card!(
    index = index::NINJA_S_KUNAI,
    oracle_id = "fa5aefaf-432f-41d0-8e54-0e8b7bc7ba98",
    scryfall_id = "af6ae567-c77a-416a-b609-4df3d0dd2e18",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ninja's Kunai",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    abilities = &[
        equip!(cost!("{1}")),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&NINJA_S_KUNAI_SHOT)
        ),
    ],
);
