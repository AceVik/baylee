//! Keldon Megaliths — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: Hellbent — {1}{R}, {T}: This land deals 1 damage to any target. Activate only if you have no cards in hand.
//! Set: JVC #58 — Duel Decks Anthology: Jace vs. Chandra | Scryfall ID: da7c4600-1dc3-4a9d-a112-1b70abcb8951 | Oracle ID: ec0ea7f7-52ce-40d1-b34c-e36dd4b26120
// PARTIAL — the land arrives tapped and taps for {R}; the Hellbent ability
// is not built, because its gate is a sentence `Condition` cannot say.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KELDON_MEGALITHS,
    oracle_id = "ec0ea7f7-52ce-40d1-b34c-e36dd4b26120",
    scryfall_id = "da7c4600-1dc3-4a9d-a112-1b70abcb8951",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Keldon Megaliths",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "Condition has no variant for \"you have no cards in hand\": the \
         Hellbent ability is omitted rather than activating unconditionally",
    ),
    // NOT SUPPORTED: "Hellbent — {1}{R}, {T}: This land deals 1 damage to any
    // target. Activate only if you have no cards in hand." The damage half is
    // ordinary (DealDamage to TargetSpec::AnyTarget); the gate is the missing
    // word. `Condition` counts permanents, a graveyard, counters on the source
    // and the source itself, and an empty hand is none of those, so the
    // ability comes off the card instead of being offered when it may not be
    // activated (the same rule that pulls a pitch cost off an activation).
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
);
