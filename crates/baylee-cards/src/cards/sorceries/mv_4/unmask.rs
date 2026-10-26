//! Unmask — {3}{B} — Sorcery
//! Oracle: You may exile a black card from your hand rather than pay this spell's mana cost.
//! Oracle: Target player reveals their hand. You choose a nonland card from it. That player discards that card.
//! Set: MMQ #168 — Mercadian Masques | Scryfall ID: 2db7a0e6-eea5-4fa6-ac14-401411b106cc | Oracle ID: 1a3a274d-41da-47db-ad41-5720f36a7963
// NOT SUPPORTED: Alternate cost of exiling a black card from your hand is not supported; spell effect works when cast for {3}{B}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNMASK,
    oracle_id = "1a3a274d-41da-47db-ad41-5720f36a7963",
    scryfall_id = "2db7a0e6-eea5-4fa6-ac14-401411b106cc",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial("alternate exile-black-card cost not supported"),
    faces = &[face!(
        name = "Unmask",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::RevealHandDiscard {
            filter: &Filter::NONLAND,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    ),],
);
