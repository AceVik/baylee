//! Aladdin's Lamp — {10} — Artifact
//! Oracle: {X}, {T}: The next time you would draw a card this turn, instead look at the top X cards of your library, put all but one of them on the bottom of your library in a random order, then draw a card. X can't be 0.
//! Set: 4ED #291 — Fourth Edition | Scryfall ID: 42e7cf40-c136-4fcb-a947-558b713b39f6 | Oracle ID: 2aa2f96b-5784-4767-b9ea-b8d9222cb1de
// PARTIAL — the draw-replacement ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALADDIN_S_LAMP,
    oracle_id = "2aa2f96b-5784-4767-b9ea-b8d9222cb1de",
    scryfall_id = "42e7cf40-c136-4fcb-a947-558b713b39f6",
    faces = &[face!(
        name = "Aladdin's Lamp",
        mana_cost = mana!("{10}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no replacement rewrites the next draw this turn (Effect::LookAtTopPick acts \
         immediately and is not that replacement), and an activation's {X} cannot be \
         bounded below 1",
    ),
    // NOT SUPPORTED: "{X}, {T}: The next time you would draw a card this turn,
    // instead look at the top X cards of your library, put all but one of them
    // on the bottom of your library in a random order, then draw a card. X
    // can't be 0." — no ReplacementRule rewrites a future draw: the only
    // draw-related one is MaySkipDrawStepDraw, and Effect::LookAtTopPick
    // { count: X, pick: 1, random: true }, the nearest shape, performs the
    // look as it resolves and moves the kept card to the hand instead of
    // replacing a draw. The activation's {X} is also unbounded below, the
    // "X can't be 0" gap the cookbook already records for Lair of the Hydra.
);
