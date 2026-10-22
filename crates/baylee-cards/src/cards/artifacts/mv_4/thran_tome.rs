//! Thran Tome — {4} — Artifact — Book
//! Oracle: {5}, {T}: Reveal the top three cards of your library. Target opponent chooses one of those cards. Put that card into your graveyard, then draw two cards.
//! Set: WTH #160 — Weatherlight | Scryfall ID: 63db7360-fe6e-430f-bfee-a2f80bcb6fec | Oracle ID: 78d2a3fd-864c-49f3-b531-c71dc3b35977
// PARTIAL — Targets an opponent, mills one card, and draws two cards; having target opponent choose which of the top three revealed cards goes to your graveyard is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THRAN_TOME,
    oracle_id = "78d2a3fd-864c-49f3-b531-c71dc3b35977",
    scryfall_id = "63db7360-fe6e-430f-bfee-a2f80bcb6fec",
    coverage = Coverage::Partial(
        "Targets an opponent, mills one card, and draws two cards; having target opponent choose which of the top three revealed cards goes to your graveyard is not yet modeled.",
    ),
    faces = &[face!(
        name = "Thran Tome",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::BOOK],
    ),],
    abilities = &[activated!(
        cost!("{5}", TapSelf),
        &[
            Effect::Mill {
                target: PlayerRel::You,
                amount: Amount::Fixed(1),
            },
            Effect::draw(2),
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    ),],
);
