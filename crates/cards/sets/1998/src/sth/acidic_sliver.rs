//! Acidic Sliver — {B}{R} — Creature — Sliver
//! Oracle: All Slivers have "{2}, Sacrifice this permanent: This permanent deals 2 damage to any target."
//! Set: TPR #206 — Tempest Remastered | Scryfall ID: d0b3ecf1-5c00-4aa0-92fd-54829495a3cf | Oracle ID: a149eeb1-007c-435d-9d55-34af205090e0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ACIDIC_ABILITY: AbilityDef = activated!(
    cost!("{2}", SacrificeSelf),
    &[Effect::DealDamage {
        amount: Amount::Fixed(2),
        target: TargetSpec::AnyTarget,
    }],
    target = Some(TargetSpec::AnyTarget),
);

card!(
    index = index::ACIDIC_SLIVER,
    oracle_id = "a149eeb1-007c-435d-9d55-34af205090e0",
    scryfall_id = "d0b3ecf1-5c00-4aa0-92fd-54829495a3cf",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Acidic Sliver",
        mana_cost = mana!("{B}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        Filter::HasSubtype(subtypes::creature::SLIVER),
        Modifier::GrantActivatedDefinition(&ACIDIC_ABILITY),
    )],
);
