//! Glyph of Doom — {B} — Instant
//! Oracle: Choose target Wall creature. At this turn's next end of combat, destroy all creatures that were blocked by that creature this turn.
//! Set: LEG #100 — Legends | Scryfall ID: 332bfce9-052d-42e9-a407-4a1dd59e0f2a | Oracle ID: a3b3e02f-c75d-4cd6-8c57-d85ce00975fb
// PARTIAL — the whole spell is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLYPH_OF_DOOM,
    oracle_id = "a3b3e02f-c75d-4cd6-8c57-d85ce00975fb",
    scryfall_id = "332bfce9-052d-42e9-a407-4a1dd59e0f2a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Effect::AtEndOfCombat and the target Wall exist, but no Filter \
         names the creatures that were blocked by that remembered Wall this \
         turn, so the delayed destroy has no filter to sweep"
    ),
    faces = &[face!(
        name = "Glyph of Doom",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "Choose target Wall creature. At this turn's next end
    // of combat, destroy all creatures that were blocked by that creature
    // this turn." — the delayed window is
    // `Effect::AtEndOfCombat { about, effects }`, but the destroy's set needs
    // a filter for "creatures blocked by that creature this turn"; blocking
    // is only ever asked of a creature's own state
    // (`Filter::Blocking`), no filter references the remembered Wall, and no
    // record of a block that has since ended can be read. Targeting the Wall
    // alone would be a spell that does nothing, so the spell comes off the
    // card.
    abilities = &[],
);
