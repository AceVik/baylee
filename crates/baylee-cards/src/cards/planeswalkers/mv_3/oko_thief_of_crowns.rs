//! Oko, Thief of Crowns — {1}{G}{U} — Legendary Planeswalker — Oko
//! Oracle: +2: Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
//! Oracle: +1: Target artifact or creature loses all abilities and becomes a green Elk creature with base power and toughness 3/3.
//! Oracle: −5: Exchange control of target artifact or creature you control and target creature an opponent controls with power 3 or less.
//! Set: ELD #197 — Throne of Eldraine | Scryfall ID: 3462a3d0-5552-49fa-9eb7-100960c55891 | Oracle ID: 60c60923-ff1b-43f7-8768-731499fcffc9
// PARTIAL — the +2 creates a Food token; the +1 and the −5 are not sayable and
// are dropped, each with the printed sentence written out below.

use crate::tokens::FOOD;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OKO_THIEF_OF_CROWNS,
    oracle_id = "60c60923-ff1b-43f7-8768-731499fcffc9",
    scryfall_id = "3462a3d0-5552-49fa-9eb7-100960c55891",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Oko, Thief of Crowns",
        mana_cost = mana!("{1}{G}{U}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::OKO],
        loyalty = Some(4),
    ),],
    coverage = Coverage::Partial(
        "the +1 needs a modifier that removes every ability (LoseKeywords strips keywords only) and one that replaces creature types; the −5 needs a two-target control exchange, and loyalty abilities carry no second target",
    ),
    abilities = &[
        // +2: Create a Food token.
        loyalty!(2, &[Effect::CreateToken { token: &FOOD }]),
        // NOT SUPPORTED: "+1: Target artifact or creature loses all abilities and
        // becomes a green Elk creature with base power and toughness 3/3." —
        // Modifier::LoseKeywords strips keyword abilities only, no modifier or
        // effect removes every ability a permanent has, and one continuous effect
        // carries one layer, so the single printed "becomes" sentence has no
        // spelling.
        // NOT SUPPORTED: "−5: Exchange control of target artifact or creature you
        // control and target creature an opponent controls with power 3 or less." —
        // Effect::ExchangeControlOrSacrifice exchanges the source with one target
        // (and sacrifices the source), not two chosen permanents. The second
        // target's filter is sayable (`Filter::PowerAtMost(3)`).
    ],
);
