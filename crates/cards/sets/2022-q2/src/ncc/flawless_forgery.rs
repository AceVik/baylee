//! Flawless Forgery — {3}{U}{U} — Sorcery
//! Oracle: Casualty 3 (As you cast this spell, you may sacrifice a creature with power 3 or greater. When you do, copy this spell and you may choose a new target for the copy.)
//! Oracle: Exile target instant or sorcery card from an opponent's graveyard. Copy that card. You may cast the copy without paying its mana cost.
//! Set: NCC #26 — New Capenna Commander | Scryfall ID: 1c808629-ee2b-4f3e-a316-7182fbc38d34 | Oracle ID: 0e3cb267-5704-4708-94c8-e2fbce4978af
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLAWLESS_FORGERY,
    oracle_id = "0e3cb267-5704-4708-94c8-e2fbce4978af",
    scryfall_id = "1c808629-ee2b-4f3e-a316-7182fbc38d34",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Flawless Forgery",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
