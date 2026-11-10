//! False Demise — {2}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature dies, return that card to the battlefield under your control.
//! Set: MMQ #80 — Mercadian Masques | Scryfall ID: 48872422-895f-45f0-ba2a-7cd307285c7d | Oracle ID: 50ffa9f0-fb3b-4de4-b463-6d7ee8b5e93a
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FALSE_DEMISE,
    oracle_id = "50ffa9f0-fb3b-4de4-b463-6d7ee8b5e93a",
    scryfall_id = "48872422-895f-45f0-ba2a-7cd307285c7d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "False Demise",
        mana_cost = mana!("{2}{U}"),
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
        triggered!(
            Trigger::Dies(&Filter::AttachedToBySource),
            &[Effect::reanimate(TargetSpec::EventObject)],
        ),
    ],
);
