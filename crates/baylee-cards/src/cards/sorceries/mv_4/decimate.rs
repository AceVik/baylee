//! Decimate — {2}{R}{G} — Sorcery
//! Oracle: Destroy target artifact, target creature, target enchantment, and target land. (You can't cast this spell unless you have legal choices for all its targets.)
//! Set: OTC #220 — Outlaws of Thunder Junction Commander | Scryfall ID: cf0e68bc-8de8-4ff8-a39a-8da95f71d843 | Oracle ID: a4e5693f-12a0-451e-818d-d6efc7b4ed25
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DECIMATE,
    oracle_id = "a4e5693f-12a0-451e-818d-d6efc7b4ed25",
    scryfall_id = "cf0e68bc-8de8-4ff8-a39a-8da95f71d843",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Decimate",
        mana_cost = mana!("{2}{R}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
