//! Whispers of the Muse — {U} — Instant
//! Oracle: Buyback {5} (You may pay an additional {5} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Draw a card.
//! Set: TPR #79 — Tempest Remastered | Scryfall ID: 5852a174-eef6-4c06-abcd-fd90f4b8a188 | Oracle ID: a4a8af4e-8686-45ba-94fd-ccf6cc8adaae
// PARTIAL — Draws a card; Buyback {5} (paying an additional {5} as you cast this spell to put it into your hand as it resolves) is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WHISPERS_OF_THE_MUSE,
    oracle_id = "a4a8af4e-8686-45ba-94fd-ccf6cc8adaae",
    scryfall_id = "5852a174-eef6-4c06-abcd-fd90f4b8a188",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Draws a card; Buyback {5} (paying an additional {5} as you cast this spell to put it into your hand as it resolves) is not yet modeled."
    ),
    faces = &[face!(
        name = "Whispers of the Muse",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::draw(1)])],
);
