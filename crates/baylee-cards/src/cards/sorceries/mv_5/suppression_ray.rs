//! Suppression Ray // Orderly Plaza — {3}{W/U}{W/U} — Sorcery // Land
//! Oracle: Tap all creatures target player controls. You may pay any amount of {E}. If you do, choose up to that many creatures tapped this way. Put a stun counter on each of them. (If a permanent with a stun counter would become untapped, remove one from it instead.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W} or {U}.
//! Set: MH3 #260 — Modern Horizons 3 | Scryfall ID: 0cccd328-457a-48ab-97fb-4bc319db2e60 | Oracle ID: b592568b-11b0-4081-90a7-30cfb9c1ba80
//! Face: Suppression Ray — {3}{W/U}{W/U} — Sorcery
//! Face: Orderly Plaza —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = 28388,
    oracle_id = "b592568b-11b0-4081-90a7-30cfb9c1ba80",
    scryfall_id = "0cccd328-457a-48ab-97fb-4bc319db2e60",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[
        face!(
            name = "Suppression Ray",
            mana_cost = mana!("{3}{W/U}{W/U}"),
            types = TypeSet::SORCERY,
        ),
        face!(name = "Orderly Plaza", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
