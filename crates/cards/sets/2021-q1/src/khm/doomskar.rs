//! Doomskar — {3}{W}{W} — Sorcery
//! Oracle: Destroy all creatures.
//! Oracle: Foretell {1}{W}{W} (During your turn, you may pay {2} and exile this card from your hand face down. Cast it on a later turn for its foretell cost.)
//! Set: KHM #9 — Kaldheim | Scryfall ID: 130ee895-1e5e-4f82-bb66-e1275bac75dd | Oracle ID: 38ece0fa-2c94-4eae-b6ab-74b783862f3e
// IMPLEMENTED — hand-authored; both ordinary and foretold casts are played in tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DOOMSKAR,
    oracle_id = "38ece0fa-2c94-4eae-b6ab-74b783862f3e",
    scryfall_id = "130ee895-1e5e-4f82-bb66-e1275bac75dd",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Doomskar",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::SORCERY,
        foretell = Some(mana!("{1}{W}{W}")),
    ),],
    abilities = &[spell!(&[Effect::DestroyAll {
        filter: &Filter::CREATURE,
        no_regen: false,
    }]),],
);
