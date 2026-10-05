//! Glyph of Reincarnation — {G} — Instant
//! Oracle: Cast this spell only after combat.
//! Oracle: Destroy all creatures that were blocked by target Wall this turn. They can't be regenerated. For each creature that died this way, put a creature card from the graveyard of the player who controlled that creature the last time it became blocked by that Wall onto the battlefield under its owner's control.
//! Set: LEG #189 — Legends | Scryfall ID: a67e8214-a192-4143-9d5e-d0e254e1bf6e | Oracle ID: 93d9a1c6-9f6f-487f-a46c-bfd80946ccdf
// PARTIAL — the whole spell is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLYPH_OF_REINCARNATION,
    oracle_id = "93d9a1c6-9f6f-487f-a46c-bfd80946ccdf",
    scryfall_id = "a67e8214-a192-4143-9d5e-d0e254e1bf6e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no Filter reads the block history (\"creatures that were blocked by \
         target Wall this turn\"), no Condition names \"only after combat\", \
         and neither the per-creature controller's graveyard nor a \
         one-card-per-died-creature reanimation has an effect"
    ),
    faces = &[face!(
        name = "Glyph of Reincarnation",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "Cast this spell only after combat." — `Condition` has
    // `BeforeStep`, `DuringStep` and `DuringCombat` but no window that opens
    // after the combat phase, so the restriction cannot be stated.
    // NOT SUPPORTED: "Destroy all creatures that were blocked by target Wall
    // this turn. They can't be regenerated." — no `Filter` names the
    // creatures blocked by a chosen permanent (blocking is read only as a
    // creature's own state, and never with history), so the sweep cannot
    // name its set.
    // NOT SUPPORTED: "For each creature that died this way, put a creature
    // card from the graveyard of the player who controlled that creature the
    // last time it became blocked by that Wall onto the battlefield under its
    // owner's control." — no effect counts the sweep's dead, none reads the
    // controller a destroyed creature had when it was blocked, and none
    // offers a per-player graveyard choice tied to such a count.
    abilities = &[],
);
