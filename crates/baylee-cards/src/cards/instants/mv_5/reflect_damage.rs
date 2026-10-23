//! Reflect Damage — {3}{R}{W} — Instant
//! Oracle: The next time a source of your choice would deal damage this turn, that damage is dealt to that source's controller instead.
//! Set: MIR #277 — Mirage | Scryfall ID: 3a2bf39b-9665-426b-b618-eb731d24a1ee | Oracle ID: 21565b0d-f814-49ca-8613-f40040c4ba6c
// PARTIAL: Effect::ReflectNextChosenSourceDamage is Eye for an Eye's sentence: the chosen source's next damage to you is not redirected, so you still take it and its controller takes the same amount from this spell as well; the printed card moves the damage onto that controller instead and covers damage to any recipient, not only to you.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REFLECT_DAMAGE,
    oracle_id = "21565b0d-f814-49ca-8613-f40040c4ba6c",
    scryfall_id = "3a2bf39b-9665-426b-b618-eb731d24a1ee",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    coverage = Coverage::Partial(
        "Effect::ReflectNextChosenSourceDamage is Eye for an Eye's sentence: the chosen source's next damage to you is not redirected, so you still take it and its controller takes the same amount from this spell as well; the printed card moves the damage onto that controller instead and covers damage to any recipient, not only to you",
    ),
    faces = &[face!(
        name = "Reflect Damage",
        mana_cost = mana!("{3}{R}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::ReflectNextChosenSourceDamage])],
);
