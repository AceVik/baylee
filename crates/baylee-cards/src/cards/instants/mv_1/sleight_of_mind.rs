//! Sleight of Mind — {U} — Instant
//! Oracle: Change the text of target spell or permanent by replacing all instances of one color word with another. (For example, you may change "target black spell" to "target blue spell." This effect lasts indefinitely.)
//! Set: 5ED #124 — Fifth Edition | Scryfall ID: 3cdb4f1c-6754-4269-8fac-d4edc02c8e00 | Oracle ID: 99dba614-40d3-41c1-a3b2-edc8777b010f
// PARTIAL — semantic text changes await independent and live acceptance.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SLEIGHT_OF_MIND,
    oracle_id = "99dba614-40d3-41c1-a3b2-edc8777b010f",
    scryfall_id = "3cdb4f1c-6754-4269-8fac-d4edc02c8e00",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial("semantic text changes await independent and live acceptance"),
    faces = &[face!(
        name = "Sleight of Mind",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ChangeTextWord {
            kind: TextWordKind::Color
        }],
        targets = Some(TargetReq::one(TargetSpec::StackOrBattlefield(&Filter::Any))),
    )],
);
