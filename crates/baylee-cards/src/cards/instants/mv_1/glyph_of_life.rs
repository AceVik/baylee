//! Glyph of Life — {W} — Instant
//! Oracle: Choose target Wall creature. Whenever that creature is dealt damage by an attacking creature this turn, you gain that much life.
//! Set: LEG #15 — Legends | Scryfall ID: ba1384e5-d140-4074-9548-250af09cb413 | Oracle ID: ec450179-35e0-4d72-b42f-b507cbce03ad
// PARTIAL — nothing is implemented: no effect registers the delayed
// "whenever that creature is dealt damage by an attacking creature this turn"
// trigger the spell creates.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLYPH_OF_LIFE,
    oracle_id = "ec450179-35e0-4d72-b42f-b507cbce03ad",
    scryfall_id = "ba1384e5-d140-4074-9548-250af09cb413",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "no effect registers the printed delayed damage trigger; granting the Wall \
         Trigger::DealtDamage cannot say \"by an attacking creature\" and the life would go \
         to the Wall's controller, not the caster"
    ),
    faces = &[face!(
        name = "Glyph of Life",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "Whenever that creature is dealt damage by an attacking
    // creature this turn, you gain that much life." — no effect registers a
    // delayed damage trigger, and the nearest shape (granting the Wall
    // Trigger::DealtDamage) has no source filter and pays its controller.
);
