//! Serpent Generator — {6} — Artifact
//! Oracle: {4}, {T}: Create a 1/1 colorless Snake artifact creature token. It has "Whenever this creature deals damage to a player, that player gets a poison counter." (A player with ten or more poison counters loses the game.)
//! Set: ME1 #164 — Masters Edition | Scryfall ID: 5e2ee41c-5592-42bd-8db2-92b3233b1d61 | Oracle ID: 0e3a11f4-c880-4932-bc7e-7aea8a9472bc
// PARTIAL — the token-making ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SERPENT_GENERATOR,
    oracle_id = "0e3a11f4-c880-4932-bc7e-7aea8a9472bc",
    scryfall_id = "5e2ee41c-5592-42bd-8db2-92b3233b1d61",
    faces = &[face!(
        name = "Serpent Generator",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "`crate::tokens` has no 1/1 colorless Snake artifact creature token and \
         a card file may not define its own TokenDef; the token's printed \
         ability is also unwritable because no Effect gives a player poison \
         counters (only the toxic keyword does, and it cannot be granted)"
    ),
    // NOT SUPPORTED: "{4}, {T}: Create a 1/1 colorless Snake artifact
    // creature token. It has \"Whenever this creature deals damage to a
    // player, that player gets a poison counter.\" (A player with ten or more
    // poison counters loses the game.)" — the cost and token creation are
    // sayable (`cost!("{4}", TapSelf)` and `Effect::CreateToken`), but the
    // pool registers no such token and a card file may not define its own
    // `TokenDef`; and nothing gives a player poison counters — only the toxic
    // keyword does, in `finish_combat_damage`, so the token's quoted ability
    // cannot be granted either.
);
