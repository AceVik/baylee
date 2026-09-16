//! Fire // Ice — {1}{R} — Instant // Instant
//! Oracle: Fire deals 2 damage divided as you choose among one or two targets.
//! Oracle: Tap target permanent.
//! Oracle: Draw a card.
//! Set: DMR #215 — Dominaria Remastered | Scryfall ID: 18303862-4726-4136-814f-157aa7006579 | Oracle ID: ae92942b-919c-4ea9-b693-85fcef765d5a
//! Face: Fire — {1}{R} — Instant
//! Face: Ice — {1}{U} — Instant
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIRE,
    oracle_id = "ae92942b-919c-4ea9-b693-85fcef765d5a",
    scryfall_id = "18303862-4726-4136-814f-157aa7006579",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[
        face!(
            name = "Fire",
            mana_cost = mana!("{1}{R}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Ice",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::INSTANT,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
