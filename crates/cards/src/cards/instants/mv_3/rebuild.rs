//! Rebuild — {2}{U} — Instant
//! Oracle: Return all artifacts to their owners' hands.
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: MH1 #66 — Modern Horizons | Scryfall ID: fd683b0b-1ac1-4fa7-9657-d3b29db2ae51 | Oracle ID: 496999a7-a82a-45ea-9c4e-1b6aabc781cd
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REBUILD,
    oracle_id = "496999a7-a82a-45ea-9c4e-1b6aabc781cd",
    scryfall_id = "fd683b0b-1ac1-4fa7-9657-d3b29db2ae51",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rebuild",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(&[Effect::ReturnAllToHand {
            filter: &Filter::ARTIFACT,
            opponents_only: false,
        }]),
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand,
        ),
    ],
);
