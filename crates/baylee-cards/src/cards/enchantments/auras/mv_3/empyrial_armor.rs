//! Empyrial Armor — {1}{W}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+1 for each card in your hand.
//! Set: VMA #25 — Vintage Masters | Scryfall ID: 6619db75-4d32-4c7b-856c-00aa33434342 | Oracle ID: ec9e8611-e785-4d5d-b6e1-bf63daf408df
// NOT SUPPORTED: giving enchanted creature +1/+1 for each card in your hand requires a hand-size count in PtCount.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMPYRIAL_ARMOR,
    oracle_id = "ec9e8611-e785-4d5d-b6e1-bf63daf408df",
    scryfall_id = "6619db75-4d32-4c7b-856c-00aa33434342",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "enchants target creature; giving enchanted creature +1/+1 for each card in your hand requires a hand-size count in PtCount",
    ),
    faces = &[face!(
        name = "Empyrial Armor",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
