//! Ghitu Fire — {X}{R} — Sorcery
//! Oracle: You may cast this spell as though it had flash if you pay {2} more to cast it. (You may cast it any time you could cast an instant.)
//! Oracle: Ghitu Fire deals X damage to any target.
//! Set: INV #144 — Invasion | Scryfall ID: 78827acd-a526-411b-bd22-ab9b538c75dd | Oracle ID: c15ae9d8-b423-4628-ad88-3f44515e0c77
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GHITU_FIRE,
    oracle_id = "c15ae9d8-b423-4628-ad88-3f44515e0c77",
    scryfall_id = "78827acd-a526-411b-bd22-ab9b538c75dd",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Ghitu Fire",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
