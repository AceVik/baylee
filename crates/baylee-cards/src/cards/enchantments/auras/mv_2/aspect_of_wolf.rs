//! Aspect of Wolf — {1}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +X/+Y, where X is half the number of Forests you control, rounded down, and Y is half the number of Forests you control, rounded up.
//! Set: 5ED #278 — Fifth Edition | Scryfall ID: 38af8356-2d7f-4699-9e57-08906c1c831b | Oracle ID: 77b7277d-90a1-4774-a998-8c35c3f94e4a
// PARTIAL — half a count, rounded down and rounded up, is not in the DSL; the
// Aura attaches and does nothing.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ASPECT_OF_WOLF,
    oracle_id = "77b7277d-90a1-4774-a998-8c35c3f94e4a",
    scryfall_id = "38af8356-2d7f-4699-9e57-08906c1c831b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "half a count, rounded down and rounded up, is not in the DSL; the Aura attaches and does nothing"
    ),
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
        // NOT SUPPORTED: Enchanted creature gets +X/+Y, where X is half the number of
        // Forests you control, rounded down, and Y is half the number of Forests you
        // control, rounded up.
    ],
);
