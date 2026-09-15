//! Bloodsoaked Insight // Sanguine Morass — {5}{B/R}{B/R} — Sorcery // Land
//! Oracle: This spell costs {1} less to cast for each 1 life your opponents have lost this turn.
//! Oracle: Target opponent exiles the top three cards of their library. Until the end of your next turn, you may play those cards. If you cast a spell this way, mana of any type can be spent to cast it.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Set: MH3 #252 — Modern Horizons 3 | Scryfall ID: 0a08e0d2-1e60-47f5-9228-4c11a127089d | Oracle ID: c52fc8a1-43c6-41f8-b010-03be7c89ef1d
//! Face: Bloodsoaked Insight — {5}{B/R}{B/R} — Sorcery
//! Face: Sanguine Morass —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLOODSOAKED_INSIGHT,
    oracle_id = "c52fc8a1-43c6-41f8-b010-03be7c89ef1d",
    scryfall_id = "0a08e0d2-1e60-47f5-9228-4c11a127089d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[
        face!(
            name = "Bloodsoaked Insight",
            mana_cost = mana!("{5}{B/R}{B/R}"),
            types = TypeSet::SORCERY,
        ),
        face!(name = "Sanguine Morass", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
