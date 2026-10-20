//! Errantry — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +3/+0 and can only attack alone.
//! Set: ME2 #124 — Masters Edition II | Scryfall ID: 01624057-cae4-42e8-b61b-34543c09488d | Oracle ID: fee455f2-fcc4-47b1-a533-92fa1ae8ae63
// PARTIAL: can-only-attack-alone restriction not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: ... and can only attack alone.
card!(
    index = index::ERRANTRY,
    oracle_id = "fee455f2-fcc4-47b1-a533-92fa1ae8ae63",
    scryfall_id = "01624057-cae4-42e8-b61b-34543c09488d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial("can-only-attack-alone restriction not supported"),
    faces = &[face!(
        name = "Errantry",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(3, 0)),
    ],
);
