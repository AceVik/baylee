//! Falling Star — {2}{R} — Sorcery
//! Oracle: Flip Falling Star onto the playing area from a height of at least one foot. Falling Star deals 3 damage to each creature it lands on. Tap all creatures dealt damage by Falling Star. If Falling Star doesn't turn completely over at least once during the flip, it has no effect.
//! Set: LEG #145 — Legends | Scryfall ID: f2b9983e-20d4-4d12-9e2c-ec6d9a345787 | Oracle ID: f5ca7b13-8003-4361-b827-7095c89f2750
// PARTIAL — no clause is written: the card is a physical flip.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FALLING_STAR,
    oracle_id = "f5ca7b13-8003-4361-b827-7095c89f2750",
    scryfall_id = "f2b9983e-20d4-4d12-9e2c-ec6d9a345787",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Falling Star",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "the card is a physical flip: which creatures it lands on and whether \
         it turns over are facts of the table, and the DSL has no effect for \
         either",
    ),
    // NOT SUPPORTED: "Flip Falling Star onto the playing area from a height of
    // at least one foot." — a dexterity instruction about the physical card;
    // no effect models it.
    // NOT SUPPORTED: "Falling Star deals 3 damage to each creature it lands
    // on. Tap all creatures dealt damage by Falling Star. If Falling Star
    // doesn't turn completely over at least once during the flip, it has no
    // effect." — the set of creatures and whether the flip happened are facts
    // of the physical flip: no effect reads where a card lands or whether it
    // turned over, so the damage, the tap and the "no effect" rider have
    // nothing to branch on. `Effect::DealDamageEach` would hit every creature,
    // which is a different card.
    abilities = &[],
);
