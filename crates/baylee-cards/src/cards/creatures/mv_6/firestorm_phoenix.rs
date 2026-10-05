//! Firestorm Phoenix — {4}{R}{R} — Creature — Phoenix
//! Oracle: Flying
//! Oracle: If this creature would die, return it to its owner's hand instead. Until that player's next turn, that player plays with that card revealed in their hand and can't play it.
//! Set: ME3 #99 — Masters Edition III | Scryfall ID: 12e70195-bad7-47d7-a2ca-4d727faa6883 | Oracle ID: 5653b40f-c566-4a14-b188-a9268ea36218
// PARTIAL — flying is written; the death replacement and the revealed-hand clause are off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FIRESTORM_PHOENIX,
    oracle_id = "5653b40f-c566-4a14-b188-a9268ea36218",
    scryfall_id = "12e70195-bad7-47d7-a2ca-4d727faa6883",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "the die replacement has no `ReplacementRule` variant — none \
         returns the card to a hand, and none leaves a card revealed in \
         its owner's hand while forbidding that player to play it"
    ),
    faces = &[face!(
        name = "Firestorm Phoenix",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHOENIX],
        power = Some(3),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: "If this creature would die, return it to its owner's
    // hand instead. Until that player's next turn, that player plays with
    // that card revealed in their hand and can't play it." —
    // `ReplacementRule` has no "would die → return to hand" variant (its
    // only self-replacement is `ExileSelfInsteadOfGraveyard`, on the
    // graveyard destination, and Time Vault's turn skip), a `Trigger::Dies`
    // fires after the death rather than replacing it, and no `Modifier`
    // reveals a card in its owner's hand or stops that player from playing
    // one (`RevealLibraryTop` reveals a library, and a cast lock is
    // player-wide, not per card).
    abilities = &[],
);
