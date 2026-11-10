//! Angelic Renewal — {1}{W} — Enchantment
//! Oracle: Whenever a creature is put into your graveyard from the battlefield, you may sacrifice this enchantment. If you do, return that card to the battlefield.
//! Set: UMA #10 — Ultimate Masters | Scryfall ID: a03fc1f1-31f6-4e52-87d0-e3d18ea60d3b | Oracle ID: f86bcdde-8b1f-4efa-a01d-39e09dc826d0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static YOUR_OWNED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::OwnedByYou]);

card!(
    index = index::ANGELIC_RENEWAL,
    oracle_id = "f86bcdde-8b1f-4efa-a01d-39e09dc826d0",
    scryfall_id = "a03fc1f1-31f6-4e52-87d0-e3d18ea60d3b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Angelic Renewal",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&YOUR_OWNED_CREATURE),
        &[Effect::MayDo {
            effects: &[Effect::SacrificeSelfThen {
                then: &[Effect::reanimate(TargetSpec::EventObject)],
            }],
        }],
    )],
);
