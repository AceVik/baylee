//! Blight — {B}{B} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: When enchanted land becomes tapped, destroy it.
//! Set: ME1 #61 — Masters Edition | Scryfall ID: d857076b-d130-482b-b015-37c22f0bdd43 | Oracle ID: 19066c43-eccd-461b-9695-c3fad95dc1da
// IMPLEMENTED — enchant land; when it becomes tapped, destroy it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLIGHT,
    oracle_id = "19066c43-eccd-461b-9695-c3fad95dc1da",
    scryfall_id = "d857076b-d130-482b-b015-37c22f0bdd43",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blight",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::LAND)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
        ),
        triggered!(
            Trigger::BecomesTapped(&Filter::AttachedToBySource),
            &[Effect::destroy(TargetSpec::EventObject)]
        ),
    ],
);
