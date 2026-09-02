//! Shatterskull Smashing // Shatterskull, the Hammer Pass — {X}{R}{R} — Sorcery // Land
//! Set: ZNR #161 — Zendikar Rising | Scryfall ID: bc7239ea-f8aa-4a6f-87bd-c35359635673 | Oracle ID: 78301998-fd9b-4cd5-afad-dbcb43cac2a7
//! Face: Shatterskull Smashing — {X}{R}{R} — Sorcery
//! Face: Shatterskull, the Hammer Pass —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1012,
    oracle_id: "78301998-fd9b-4cd5-afad-dbcb43cac2a7",
    scryfall_id: "bc7239ea-f8aa-4a6f-87bd-c35359635673",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Shatterskull Smashing",
        mana_cost: baylee_core::mana!("{X}{R}{R}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Shatterskull, the Hammer Pass",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
