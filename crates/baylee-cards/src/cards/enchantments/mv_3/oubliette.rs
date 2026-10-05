//! Oubliette — {1}{B}{B} — Enchantment
//! Oracle: When this enchantment enters, target creature phases out until this enchantment leaves the battlefield. Tap that creature as it phases in this way. (Auras and Equipment phase out with it. While permanents are phased out, they're treated as though they don't exist.)
//! Set: 2XM #100 — Double Masters | Scryfall ID: d4800a7d-c229-4ced-97ff-0e58645d58d6 | Oracle ID: c753e9e3-9374-4e3c-8622-94576a8c1da3
// PARTIAL — the enter trigger is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OUBLIETTE,
    oracle_id = "c753e9e3-9374-4e3c-8622-94576a8c1da3",
    scryfall_id = "d4800a7d-c229-4ced-97ff-0e58645d58d6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Oubliette",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Partial(
        "nothing phases the target back in when this enchantment leaves, and \
         nothing taps it as it phases in"
    ),
    // NOT SUPPORTED: "When this enchantment enters, target creature phases
    // out until this enchantment leaves the battlefield. Tap that creature
    // as it phases in this way." — Effect::PhaseOut phases the target out,
    // but the only phasing in the engine is the untap-step event (CR 502.1):
    // no effect brings a phased-out permanent back, none ties that return to
    // "until this enchantment leaves the battlefield"
    // (Effect::ExileUntil's SourceLeavesBattlefield is exile, a different
    // zone), and no rider taps it as it phases in.
);
