//! Amnesia — {3}{U}{U}{U} — Sorcery
//! Oracle: Target player reveals their hand and discards all nonland cards.
//! Set: ME1 #29 — Masters Edition | Scryfall ID: 23d6dbd3-652f-4dc3-9453-24299332938d | Oracle ID: 8345187a-a210-49d7-bdce-10fb2909e050

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AMNESIA,
    oracle_id = "8345187a-a210-49d7-bdce-10fb2909e050",
    scryfall_id = "23d6dbd3-652f-4dc3-9453-24299332938d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Amnesia",
        mana_cost = mana!("{3}{U}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::RevealHand {
                who: PlayerRel::Chosen,
            },
            Effect::DiscardMatching {
                who: PlayerRel::Chosen,
                filter: &Filter::NONLAND,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
