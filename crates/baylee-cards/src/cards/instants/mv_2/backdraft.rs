//! Backdraft — {1}{R} — Instant
//! Oracle: Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.
//! Set: LEG #132 — Legends | Scryfall ID: 58d5b9fe-b66a-48c9-94c4-db783e605f37 | Oracle ID: 80734bb0-5032-4355-9304-49f4a4557aba
// PARTIAL — the card's one sentence needs a per-spell damage history and a
// restricted player choice the DSL does not carry, so it has no ability.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BACKDRAFT,
    oracle_id = "80734bb0-5032-4355-9304-49f4a4557aba",
    scryfall_id = "58d5b9fe-b66a-48c9-94c4-db783e605f37",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no Amount reads \"the damage dealt by one of those sorcery spells \
         this turn\", no Amount halves a quantity (only Amount::is_negative \
         carries a sign and nothing rounds down), and no target spec offers a \
         player restricted to those who cast a sorcery spell this turn, so \
         neither the choice nor the damage is written"
    ),
    faces = &[face!(
        name = "Backdraft",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "Choose a player who cast one or more sorcery spells
    // this turn. Backdraft deals damage to that player equal to half the
    // damage dealt by one of those sorcery spells this turn, rounded down."
    // — the engine keeps no per-spell damage history this turn (the only
    // damage amount is `Amount::DamageDealtToYouThisTurn`, which sums to the
    // controller and names no source), and no `TargetSpec` narrows the player
    // choice to those who cast a sorcery this turn, so the sentence has no
    // head to hang the damage on.
);
