//! Ydwen Efreet — {R}{R}{R} — Creature — Efreet
//! Oracle: Whenever this creature blocks, flip a coin. If you lose the flip, remove this creature from combat and it can't block this turn. Creatures it was blocking that had become blocked by only this creature this combat become unblocked.
//! Set: ME1 #112 — Masters Edition | Scryfall ID: 79a0dd6d-8904-467f-b63b-13dfc45232fb | Oracle ID: 15b9b0cc-47ef-4147-aba7-a7adae41921b
// PARTIAL — the block trigger's coin-flip ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::YDWEN_EFREET,
    oracle_id = "15b9b0cc-47ef-4147-aba7-a7adae41921b",
    scryfall_id = "79a0dd6d-8904-467f-b63b-13dfc45232fb",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Ydwen Efreet",
        mana_cost = mana!("{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::EFREET],
        power = Some(3),
        toughness = Some(6),
    ),],
    coverage =
        Coverage::Partial("no effect flips a coin, so the lose branch has nothing that chooses it"),
    // NOT SUPPORTED: "Whenever this creature blocks, flip a coin. If you lose
    // the flip, remove this creature from combat and it can't block this turn.
    // Creatures it was blocking that had become blocked by only this creature
    // this combat become unblocked." — the trigger is not quite sayable
    // (`Trigger::BlocksOrBecomesBlockedBy` also fires when this creature
    // becomes blocked), but the sentence's head is the missing piece: no
    // `Effect` or `ReplacementRule` flips a coin (the engine's
    // `GameRng::below(2)` has no vocabulary, as The Gold Saucer and Bottle of
    // Suleiman also record), and no `Effect` removes a creature from combat.
    // Without the flip the lose branch has no chooser, so the ability comes
    // off the card.
);
