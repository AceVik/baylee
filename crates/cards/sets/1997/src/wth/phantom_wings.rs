//! Phantom Wings — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has flying.
//! Oracle: Sacrifice this Aura: Return enchanted creature to its owner's hand.
//! Set: BOK #46 — Betrayers of Kamigawa | Scryfall ID: a96e5a18-5958-426e-892f-5c68f44c1b41 | Oracle ID: 9da74f0f-143d-4db2-81eb-c7873744a2d0
// PARTIAL — Enchant creature and "Enchanted creature has flying." NOT SUPPORTED: "Sacrifice this Aura: Return enchanted creature to its owner's hand." — once the Aura is sacrificed as the cost it is attached to nothing, so the creature it enchanted cannot be named; as an ability that only sacrificed the Aura it would be a button that destroys it for nothing.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTOM_WINGS,
    oracle_id = "9da74f0f-143d-4db2-81eb-c7873744a2d0",
    scryfall_id = "a96e5a18-5958-426e-892f-5c68f44c1b41",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "enchants a creature and gives it flying; the ability 'Sacrifice this Aura: Return enchanted creature to its owner's hand' is not supported (after the sacrifice cost the Aura is attached to nothing, so the creature it enchanted cannot be named)",
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
        // NOT SUPPORTED: "Sacrifice this Aura: Return enchanted creature to its
        // owner's hand." — the creature cannot be named once the Aura has left.
    ],
);
