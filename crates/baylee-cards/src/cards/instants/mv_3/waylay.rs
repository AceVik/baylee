//! Waylay — {2}{W} — Instant
//! Oracle: Create three 2/2 white Knight creature tokens. Exile them at the beginning of the next cleanup step.
//! Set: USG #56 — Urza's Saga | Scryfall ID: 867a33ee-0340-413a-8243-9d6bc2d944e2 | Oracle ID: f0db78c7-09b5-4981-8ca8-6dbab597c5d9
// PARTIAL: exiling the created tokens at the beginning of the next cleanup step is not in Effect; creates three 2/2 white Knight creature tokens.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAYLAY,
    oracle_id = "f0db78c7-09b5-4981-8ca8-6dbab597c5d9",
    scryfall_id = "867a33ee-0340-413a-8243-9d6bc2d944e2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "exiling the created tokens at the beginning of the next cleanup step is not in Effect; creates three 2/2 white Knight creature tokens",
    ),
    faces = &[face!(
        name = "Waylay",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::CreateTokenN {
        token: &crate::generated_tokens::KNIGHT_2_2_WHITE,
        amount: Amount::Fixed(3),
    }])],
);
