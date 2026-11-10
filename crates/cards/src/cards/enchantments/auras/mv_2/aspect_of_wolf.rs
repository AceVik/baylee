//! Aspect of Wolf — {1}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +X/+Y, where X is half the number of Forests you control, rounded down, and Y is half the number of Forests you control, rounded up.
//! Set: 5ED #278 — Fifth Edition | Scryfall ID: 38af8356-2d7f-4699-9e57-08906c1c831b | Oracle ID: 77b7277d-90a1-4774-a998-8c35c3f94e4a

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FORESTS: Filter = Filter::HasSubtype(subtypes::land::FOREST);

card!(
    index = index::ASPECT_OF_WOLF,
    oracle_id = "77b7277d-90a1-4774-a998-8c35c3f94e4a",
    scryfall_id = "38af8356-2d7f-4699-9e57-08906c1c831b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aspect of Wolf",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        // "You" is the Aura's controller, whoever controls the creature.
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPTHalfCount(PtCount::YouControl(&FORESTS))
        ),
    ],
);
