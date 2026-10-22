//! Phantom Wings — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has flying.
//! Oracle: Sacrifice this Aura: Return enchanted creature to its owner's hand.
//! Set: BOK #46 — Betrayers of Kamigawa | Scryfall ID: a96e5a18-5958-426e-892f-5c68f44c1b41 | Oracle ID: 9da74f0f-143d-4db2-81eb-c7873744a2d0
// PARTIAL — Enchants a creature, grants flying to enchanted creature, and has 'Sacrifice this Aura' as an activated ability; returning the enchanted creature to its owner's hand after sacrificing the Aura as a cost requires LKI attachment tracking across activation cost payment.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTOM_WINGS,
    oracle_id = "9da74f0f-143d-4db2-81eb-c7873744a2d0",
    scryfall_id = "a96e5a18-5958-426e-892f-5c68f44c1b41",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Enchants a creature, grants flying to enchanted creature, and has 'Sacrifice this Aura' as an activated ability; returning the enchanted creature to its owner's hand after sacrificing the Aura as a cost requires LKI attachment tracking across activation cost payment.",
    ),
    faces = &[face!(
        name = "Phantom Wings",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::AddKeyword(KeywordSet::FLYING),
        ),
        activated!(
            cost!(SacrificeSelf),
            &[Effect::ReturnAllToHand {
                filter: &Filter::AttachedToBySource,
                opponents_only: false,
            }],
        ),
    ],
);
