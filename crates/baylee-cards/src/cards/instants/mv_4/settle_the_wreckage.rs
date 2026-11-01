//! Settle the Wreckage — {2}{W}{W} — Instant
//! Oracle: Exile all attacking creatures target player controls. That player may search their library for that many basic land cards, put those cards onto the battlefield tapped, then shuffle.
//! Set: HOB #26 — The Hobbit | Scryfall ID: 31a7a5e2-4cb8-48fc-8351-18344b4a7560 | Oracle ID: 2a2ea189-b663-4f4a-bb23-ff7a4af25f71
// IMPLEMENTED — exile that player's attackers; optional basic-land ramp.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SETTLE_THE_WRECKAGE,
    oracle_id = "2a2ea189-b663-4f4a-bb23-ff7a4af25f71",
    scryfall_id = "31a7a5e2-4cb8-48fc-8351-18344b4a7560",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Settle the Wreckage",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::ExileAttackingForThenMaySearchBasicLands {
            player: PlayerRel::Chosen,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
