//! Winter Blast — {X}{G} — Sorcery
//! Oracle: Tap X target creatures. Winter Blast deals 2 damage to each of those creatures with flying.
//! Set: ME1 #138 — Masters Edition | Scryfall ID: d806c7c8-3aac-4385-b937-f66e360ffd28 | Oracle ID: 594827de-e85f-4aac-b0e6-d92878f0f26c

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINTER_BLAST,
    oracle_id = "594827de-e85f-4aac-b0e6-d92878f0f26c",
    scryfall_id = "d806c7c8-3aac-4385-b937-f66e360ffd28",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Winter Blast",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::TapTarget,
            Effect::DealDamageToMatchingTargets {
                amount: Amount::Fixed(2),
                filter: &Filter::HasKeyword(KeywordSet::FLYING)
            }
        ],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
