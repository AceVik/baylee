//! Starnheim Unleashed — {2}{W}{W} — Sorcery
//! Oracle: Create a 4/4 white Angel Warrior creature token with flying and vigilance. If this spell was foretold, create X of those tokens instead.
//! Oracle: Foretell {X}{X}{W} (During your turn, you may pay {2} and exile this card from your hand face down. Cast it on a later turn for its foretell cost.)
//! Set: KHM #33 — Kaldheim | Scryfall ID: ea660d42-a441-48d8-98c1-00933cde94a4 | Oracle ID: 495ebe60-f0e5-4f0f-9d99-b2c64c95dbf4
// IMPLEMENTED — hand-authored; ordinary and foretold token counts are played in tests.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::STARNHEIM_UNLEASHED,
    oracle_id = "495ebe60-f0e5-4f0f-9d99-b2c64c95dbf4",
    scryfall_id = "ea660d42-a441-48d8-98c1-00933cde94a4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Starnheim Unleashed",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::SORCERY,
        foretell = Some(mana!("{X}{X}{W}")),
    ),],
    abilities = &[spell!(&[Effect::CreateTokenN {
        token: &generated_tokens::ANGEL_WARRIOR_4_4_WHITE_FLYING_VIGILANCE,
        amount: Amount::XIfSourceWasForetold { otherwise: 1 },
    }]),],
);
