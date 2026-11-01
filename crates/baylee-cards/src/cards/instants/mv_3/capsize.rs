//! Capsize — {1}{U}{U} — Instant
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Return target permanent to its owner's hand.
//! Set: TPR #42 — Tempest Remastered | Scryfall ID: f36f6545-e4e0-4dc8-acea-e527e43f3e14 | Oracle ID: 77637eff-2963-4402-88f3-ca346f762fc8
// PARTIAL — Returns target permanent to its owner's hand; Buyback {3} (paying an additional {3} as you cast this spell to put it into your hand as it resolves) is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CAPSIZE,
    oracle_id = "77637eff-2963-4402-88f3-ca346f762fc8",
    scryfall_id = "f36f6545-e4e0-4dc8-acea-e527e43f3e14",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Returns target permanent to its owner's hand; Buyback {3} (paying an additional {3} as you cast this spell to put it into your hand as it resolves) is not yet modeled."
    ),
    faces = &[face!(
        name = "Capsize",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::bounce(TargetSpec::Object(&Filter::Any))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
    )],
);
