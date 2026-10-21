//! Awesome Presence — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked unless defending player pays {3} for each creature they control that's blocking it.
//! Set: ALL #23a — Alliances | Scryfall ID: 80017e84-87bc-4eb8-a663-5c263d8df812 | Oracle ID: d6efff61-2087-4f48-a8a8-6c465190dad9
// NOT SUPPORTED: requiring the defending player to pay {3} for each creature they control blocking enchanted creature is not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AWESOME_PRESENCE,
    oracle_id = "d6efff61-2087-4f48-a8a8-6c465190dad9",
    scryfall_id = "80017e84-87bc-4eb8-a663-5c263d8df812",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "attaches to target creature; requiring the defending player to pay {3} for each creature they control blocking enchanted creature is not supported",
    ),
    faces = &[face!(
        name = "Awesome Presence",
        mana_cost = mana!("{U}"),
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
