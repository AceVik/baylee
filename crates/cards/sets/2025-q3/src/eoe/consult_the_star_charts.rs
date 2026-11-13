//! Consult the Star Charts — {1}{U} — Instant
//! Oracle: Kicker {1}{U} (You may pay an additional {1}{U} as you cast this spell.)
//! Oracle: Look at the top X cards of your library, where X is the number of lands you control. Put one of those cards into your hand. If this spell was kicked, put two of those cards into your hand instead. Put the rest on the bottom of your library in a random order.
//! Set: EOE #51 — Edge of Eternities | Scryfall ID: a16a6555-2e3a-4587-aacd-0307d696b26c | Oracle ID: e921839f-9d91-41a9-bc89-016af3c757aa
use baylee_cards_dsl::prelude::*;

/// "Where X is the number of lands you control."
const LANDS: Amount = Amount::CountOf {
    filter: &Filter::YOUR_LAND,
    zone: ZoneSel::Battlefield,
};

card!(
    index = index::CONSULT_THE_STAR_CHARTS,
    oracle_id = "e921839f-9d91-41a9-bc89-016af3c757aa",
    scryfall_id = "a16a6555-2e3a-4587-aacd-0307d696b26c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Consult the Star Charts",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
        additional_costs = &[cost!("{1}{U}")],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::IfKicked {
        then: &[Effect::LookAtTopPick {
            count: LANDS,
            pick: 2,
            random: true,
        }],
        otherwise: &[Effect::LookAtTopPick {
            count: LANDS,
            pick: 1,
            random: true,
        }],
    }])],
);
