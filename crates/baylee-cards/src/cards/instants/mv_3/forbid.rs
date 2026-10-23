//! Forbid — {1}{U}{U} — Instant
//! Oracle: Buyback—Discard two cards. (You may discard two cards in addition to any other costs as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Counter target spell.
//! Set: TPR #50 — Tempest Remastered | Scryfall ID: 0fc2a6c5-9904-47e0-adec-e9e6e79cd11d | Oracle ID: 6e26a525-2584-490d-b106-621c1a30e1fd
// PARTIAL — Buyback is not supported; this spell only counters target spell and goes to the graveyard as it resolves.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Buyback is not supported; this spell only counters target spell and goes to the graveyard as it resolves.
card!(
    index = index::FORBID,
    oracle_id = "6e26a525-2584-490d-b106-621c1a30e1fd",
    scryfall_id = "0fc2a6c5-9904-47e0-adec-e9e6e79cd11d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Buyback is not supported; this spell only counters target spell and goes to the graveyard as it resolves."
    ),
    faces = &[face!(
        name = "Forbid",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )],
);
