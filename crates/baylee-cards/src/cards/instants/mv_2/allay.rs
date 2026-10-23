//! Allay — {1}{W} — Instant
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Destroy target enchantment.
//! Set: EXO #1 — Exodus | Scryfall ID: f20a1c6d-ec6a-4bd6-b3b2-b997f71d41fc | Oracle ID: 664d982a-5cd8-4289-afb8-fa2e083042c9
// PARTIAL — Buyback {3} is not supported; destroys target enchantment and goes to the graveyard on resolution.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Buyback {3} is not supported; destroys target enchantment and goes to the graveyard on resolution.
card!(
    index = index::ALLAY,
    oracle_id = "664d982a-5cd8-4289-afb8-fa2e083042c9",
    scryfall_id = "f20a1c6d-ec6a-4bd6-b3b2-b997f71d41fc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Buyback {3} is not supported; destroys target enchantment and goes to the graveyard on resolution."
    ),
    faces = &[face!(
        name = "Allay",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::ENCHANTMENT))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ENCHANTMENT))),
    )],
);
