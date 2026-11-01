//! Mind Peel — {B} — Sorcery
//! Oracle: Buyback {2}{B}{B} (You may pay an additional {2}{B}{B} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Target player discards a card.
//! Set: STH #63 — Stronghold | Scryfall ID: 92bd531c-fdd1-4a22-816e-258b2975c1a3 | Oracle ID: 4f424d94-1544-4086-a57d-2add17332094
// PARTIAL: target player discards a card; buyback {2}{B}{B} is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIND_PEEL,
    oracle_id = "4f424d94-1544-4086-a57d-2add17332094",
    scryfall_id = "92bd531c-fdd1-4a22-816e-258b2975c1a3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage =
        Coverage::Partial("target player discards a card; buyback {2}{B}{B} is not supported"),
    faces = &[face!(
        name = "Mind Peel",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DiscardForPlayers {
            who: PlayerRel::Chosen,
            count: 1,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
