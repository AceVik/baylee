//! Metrognome — {4} — Artifact
//! Oracle: When a spell or ability an opponent controls causes you to discard this card, create four 1/1 colorless Gnome artifact creature tokens.
//! Oracle: {4}, {T}: Create a 1/1 colorless Gnome artifact creature token.
//! Set: USG #301 — Urza's Saga | Scryfall ID: d71f153d-2d07-4a8a-b725-747bb96afe00 | Oracle ID: 57fe5c94-ee93-45c6-bf7e-89d177d0fe70
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::METROGNOME,
    oracle_id = "57fe5c94-ee93-45c6-bf7e-89d177d0fe70",
    scryfall_id = "d71f153d-2d07-4a8a-b725-747bb96afe00",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Metrognome",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::DiscardedThisByOpponent,
            &[Effect::CreateTokenN {
                token: &crate::generated_tokens::GNOME_ARTIFACT_1_1,
                amount: Amount::Fixed(4),
            }],
        ),
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::CreateToken {
                token: &crate::generated_tokens::GNOME_ARTIFACT_1_1,
            }],
        ),
    ],
);
