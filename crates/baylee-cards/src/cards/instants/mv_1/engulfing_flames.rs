//! Engulfing Flames — {R} — Instant
//! Oracle: Engulfing Flames deals 1 damage to target creature. It can't be regenerated this turn.
//! Oracle: Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #191 — Odyssey | Scryfall ID: 164a92ac-0788-4238-a2ed-1ebd3dd0dd88 | Oracle ID: 88f935ee-7dbb-4c27-8485-f56b4193fa11
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENGULFING_FLAMES,
    oracle_id = "88f935ee-7dbb-4c27-8485-f56b4193fa11",
    scryfall_id = "164a92ac-0788-4238-a2ed-1ebd3dd0dd88",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Engulfing Flames",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
