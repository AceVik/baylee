//! Taste of Paradise — {3}{G} — Sorcery
//! Oracle: As an additional cost to cast this spell, you may pay {1}{G} any number of times.
//! Oracle: You gain 3 life plus an additional 3 life for each additional {1}{G} you paid.
//! Set: ALL #100a — Alliances | Scryfall ID: b9248694-88fa-4fe1-9902-f03a41100cd6 | Oracle ID: c9a20d03-b90a-43df-8ab4-ff0b38c90a08

// PARTIAL — the {1}{G} additional cost may be paid once (3 life more); the spell gains 3 life, or 6 when it was paid.
// NOT SUPPORTED: "you may pay {1}{G} any number of times" and "an additional 3 life for each additional {1}{G} you paid." The optional additional cost is paid at most once (kicker-like), so the life gained never scales past 6; the card does less than printed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TASTE_OF_PARADISE,
    oracle_id = "c9a20d03-b90a-43df-8ab4-ff0b38c90a08",
    scryfall_id = "b9248694-88fa-4fe1-9902-f03a41100cd6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Taste of Paradise",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
        additional_costs = &[cost!("{1}{G}")],
    ),],
    coverage = Coverage::Partial(
        "The {1}{G} additional cost may be paid at most once rather than any number of times; paying it gains 6 life instead of 3."
    ),
    abilities = &[spell!(&[Effect::IfKicked {
        then: &[Effect::gain_life(6)],
        otherwise: &[Effect::gain_life(3)],
    }])],
);
