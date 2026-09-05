//! Arcane Signet — {2} — Artifact
//! Oracle: {T}: Add one mana of any color in your commander's color identity.
//! Set: MSC #191 — Marvel Super Heroes Commander | Scryfall ID: 1cad1bd2-7c56-4ce0-99a6-b2a49c1288dd | Oracle ID: 0bc7f093-bef0-4f1a-852c-4b75ebf54838
// IMPLEMENTED — mana rock whose colour is the commander's identity. The
// first card in the pool to use `ManaSource::CommanderIdentity`, which the
// engine has resolved since the mana sources were unified but nothing asked
// for; the engine test is in card_tests.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1345,
    oracle_id: "0bc7f093-bef0-4f1a-852c-4b75ebf54838",
    scryfall_id: "1cad1bd2-7c56-4ce0-99a6-b2a49c1288dd",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Arcane Signet",
        mana_cost: baylee_core::mana!("{2}"),
        types: TypeSet::ARTIFACT,
    },
    ],
    abilities: &[mana_ability!(&[Effect::mana_commander_identity()])],
}
