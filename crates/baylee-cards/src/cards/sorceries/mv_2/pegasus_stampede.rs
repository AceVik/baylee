//! Pegasus Stampede — {1}{W} — Sorcery
//! Oracle: Buyback—Sacrifice a land. (You may sacrifice a land in addition to any other costs as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Create a 1/1 white Pegasus creature token with flying.
//! Set: TPR #24 — Tempest Remastered | Scryfall ID: 7418d65f-1017-4f98-b039-db4228769454 | Oracle ID: 16b4a48a-fec2-433c-948d-e0b16c961172
// PARTIAL — Creates a 1/1 white Pegasus creature token with flying; Buyback—Sacrifice a land is not supported.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Creates a 1/1 white Pegasus creature token with flying; Buyback—Sacrifice a land is not supported.
card!(
    index = index::PEGASUS_STAMPEDE,
    oracle_id = "16b4a48a-fec2-433c-948d-e0b16c961172",
    scryfall_id = "7418d65f-1017-4f98-b039-db4228769454",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Creates a 1/1 white Pegasus creature token with flying; Buyback—Sacrifice a land is not supported.",
    ),
    faces = &[face!(
        name = "Pegasus Stampede",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::CreateToken {
        token: &crate::generated_tokens::PEGASUS_1_1_WHITE_FLYING,
    }]),],
);
