//! Repopulate — {1}{G} — Instant
//! Oracle: Shuffle all creature cards from target player's graveyard into that player's library.
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: ULG #111 — Urza's Legacy | Scryfall ID: aac77869-97bc-4976-9ee0-3d60e162b78a | Oracle ID: 3a225d39-c1f8-4ab0-98ec-dad63bf268db
// PARTIAL — cycling {2}; shuffling only creature cards from the targeted graveyard into its library is unsupported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REPOPULATE,
    oracle_id = "3a225d39-c1f8-4ab0-98ec-dad63bf268db",
    scryfall_id = "aac77869-97bc-4976-9ee0-3d60e162b78a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "cycling {2}; shuffling only creature cards from the targeted graveyard into its library is unsupported"
    ),
    faces = &[face!(
        name = "Repopulate",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: shuffle all creature cards from target player's graveyard into their library.
    abilities = &[activated!(
        cost!("{2}", DiscardSelf),
        &[Effect::draw(1)],
        zone = ActivationZone::Hand
    )],
);
