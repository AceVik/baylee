//! Wax // Wane — {G} — Instant // Instant
//! Oracle: Target creature gets +2/+2 until end of turn.
//! Oracle: Destroy target enchantment.
//! Set: DMR #211 — Dominaria Remastered | Scryfall ID: f93b22de-93f3-4f05-8960-fa6a539ea882 | Oracle ID: 4744b086-fea2-4992-bfef-68c58cdf9cf0
//! Face: Wax — {G} — Instant
//! Face: Wane — {W} — Instant
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAX,
    oracle_id = "4744b086-fea2-4992-bfef-68c58cdf9cf0",
    scryfall_id = "f93b22de-93f3-4f05-8960-fa6a539ea882",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[
        face!(
            name = "Wax",
            mana_cost = mana!("{G}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Wane",
            mana_cost = mana!("{W}"),
            types = TypeSet::INSTANT,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
