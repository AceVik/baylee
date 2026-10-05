//! Storm Seeker — {3}{G} — Instant
//! Oracle: Storm Seeker deals damage to target player equal to the number of cards in that player's hand.
//! Set: ME1 #132 — Masters Edition | Scryfall ID: 456a2fdf-e91f-4d6d-840e-562fe7f5acd3 | Oracle ID: e694bf97-9deb-44f1-9d26-264b57596346
// PARTIAL — nothing is implemented: no amount counts the cards in the
// targeted player's hand, so the damage clause cannot be written.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STORM_SEEKER,
    oracle_id = "e694bf97-9deb-44f1-9d26-264b57596346",
    scryfall_id = "456a2fdf-e91f-4d6d-840e-562fe7f5acd3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no amount counts the cards in the targeted player's hand: ZoneSel offers HandYou \
         and HandActivePlayer only"
    ),
    faces = &[face!(
        name = "Storm Seeker",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "Storm Seeker deals damage to target player equal to the
    // number of cards in that player's hand." — no Amount reads a chosen
    // player's hand (Amount::CountOf cannot name it and TargetPower-style
    // readers do not count cards).
);
