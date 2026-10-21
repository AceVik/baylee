//! Snowblind — {3}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets -X/-Y. If that creature is attacking, X is the number of snow lands defending player controls. Otherwise, X is the number of snow lands its controller controls. Y is equal to X or to enchanted creature's toughness minus 1, whichever is smaller.
//! Set: ICE #264 — Ice Age | Scryfall ID: 5f62c376-487a-42bc-bd85-ab8b0480f7dc | Oracle ID: 3f8bfa3d-7490-4259-a403-d13585ed2f66

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SNOWBLIND,
    oracle_id = "3f8bfa3d-7490-4259-a403-d13585ed2f66",
    scryfall_id = "5f62c376-487a-42bc-bd85-ab8b0480f7dc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "attaches to target creature; dynamic -X/-Y based on snow lands with Y capped at enchanted creature's toughness minus 1 is not supported, so enchanted creature's power and toughness are unchanged",
    ),
    faces = &[face!(
        name = "Snowblind",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        // CR 303.4a: Enchant creature.
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        // NOT SUPPORTED: Dynamic -X/-Y where X switches between defending player's snow lands (when attacking) and controller's snow lands (otherwise) and Y is capped at enchanted creature's toughness minus 1.
    ],
);
