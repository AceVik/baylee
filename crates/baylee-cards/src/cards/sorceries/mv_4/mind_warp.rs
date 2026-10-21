//! Mind Warp — {X}{3}{B} — Sorcery
//! Oracle: Look at target player's hand and choose X cards from it. That player discards those cards.
//! Set: 6ED #143 — Classic Sixth Edition | Scryfall ID: e1f0d58c-24fa-498f-9531-e62144401e86 | Oracle ID: 5109aa76-ca40-44d1-abb4-5e7a23a2c274
// NOT SUPPORTED: choosing X cards from target player's looked-at hand for them to discard is not supported; looks at target player's hand without discarding.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIND_WARP,
    oracle_id = "5109aa76-ca40-44d1-abb4-5e7a23a2c274",
    scryfall_id = "e1f0d58c-24fa-498f-9531-e62144401e86",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "choosing X cards from target player's looked-at hand for them to discard is not supported; looks at target player's hand without discarding",
    ),
    faces = &[face!(
        name = "Mind Warp",
        mana_cost = mana!("{X}{3}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::LookAtChosenHand],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
