//! Lab Rats — {B} — Sorcery
//! Oracle: Buyback {4} (You may pay an additional {4} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Create a 1/1 black Rat creature token.
//! Set: TPR #108 — Tempest Remastered | Scryfall ID: 6fd5d79e-cfd8-484b-bbc8-785177922e30 | Oracle ID: c53d6d3e-64e3-48b9-afab-3bde3164888f
// PARTIAL: creates a 1/1 black Rat creature token; buyback {4} is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAB_RATS,
    oracle_id = "c53d6d3e-64e3-48b9-afab-3bde3164888f",
    scryfall_id = "6fd5d79e-cfd8-484b-bbc8-785177922e30",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage =
        Coverage::Partial("creates a 1/1 black Rat creature token; buyback {4} is not supported"),
    faces = &[face!(
        name = "Lab Rats",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::CreateToken {
        token: &crate::generated_tokens::RAT_1_1_BLACK,
    }])],
);
