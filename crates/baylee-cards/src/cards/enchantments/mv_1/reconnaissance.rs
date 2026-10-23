//! Reconnaissance — {W} — Enchantment
//! Oracle: {0}: Remove target attacking creature you control from combat and untap it. (If you activate during end of combat, the creature will untap after it deals combat damage.)
//! Set: EXO #17 — Exodus | Scryfall ID: a16012d8-703c-4385-8769-13e3caba3fc6 | Oracle ID: 32599d7c-243b-447b-9d5d-e8e29830ac61
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

const YOUR_ATTACKING_CREATURE: Filter = Filter::And(&[Filter::YOUR_CREATURE, Filter::Attacking]);

card!(
    index = index::RECONNAISSANCE,
    oracle_id = "32599d7c-243b-447b-9d5d-e8e29830ac61",
    scryfall_id = "a16012d8-703c-4385-8769-13e3caba3fc6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reconnaissance",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        Cost::FREE,
        &[
            Effect::RemoveTargetFromCombat { unblock: false },
            Effect::UntapTarget,
        ],
        target = Some(TargetSpec::Object(&YOUR_ATTACKING_CREATURE))
    ),],
);
