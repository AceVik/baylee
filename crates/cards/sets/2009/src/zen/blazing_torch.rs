//! Blazing Torch — {1} — Artifact — Equipment
//! Oracle: Equipped creature can't be blocked by Vampires or Zombies.
//! Oracle: Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."
//! Oracle: Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
//! Set: INR #254 — Innistrad Remastered | Scryfall ID: ea2d3a1c-287e-4b85-8f3d-25fdcf79f47b | Oracle ID: cd3d1e5d-db12-4878-92a2-23eb8d23eaf3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLAZING_TORCH_SHOT: AbilityDef = activated!(
    cost!(TapSelf, SacrificeGrantor),
    &[Effect::GrantorDealsDamage {
        amount: Amount::Fixed(2),
        target: TargetSpec::AnyTarget,
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
);

static VAMPIRE_OR_ZOMBIE: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::creature::VAMPIRE),
    Filter::HasSubtype(subtypes::creature::ZOMBIE),
]);

card!(
    index = index::BLAZING_TORCH,
    oracle_id = "cd3d1e5d-db12-4878-92a2-23eb8d23eaf3",
    scryfall_id = "ea2d3a1c-287e-4b85-8f3d-25fdcf79f47b",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blazing Torch",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    abilities = &[
        equip!(cost!("{1}")),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::CantBeBlockedBy(&VAMPIRE_OR_ZOMBIE)
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&BLAZING_TORCH_SHOT)
        ),
    ],
);
