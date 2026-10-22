//! Relentless Assault — {2}{R}{R} — Sorcery
//! Oracle: Untap all creatures that attacked this turn. After this main phase, there is an additional combat phase followed by an additional main phase.
//! Set: CMR #416 — Commander Legends | Scryfall ID: 97622209-9357-4347-8961-f77f8d2389d8 | Oracle ID: dc1c0e8c-d0c1-445c-968f-7dec91e5d5fc
// PARTIAL: untaps all creatures that attacked this turn; adding an extra combat phase and main phase is not yet supported.

use baylee_cards_dsl::prelude::*;

static ATTACKED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttackedThisTurn]);

// NOT SUPPORTED: After this main phase, there is an additional combat phase followed by an additional main phase.
card!(
    index = index::RELENTLESS_ASSAULT,
    oracle_id = "dc1c0e8c-d0c1-445c-968f-7dec91e5d5fc",
    scryfall_id = "97622209-9357-4347-8961-f77f8d2389d8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "untaps all creatures that attacked this turn; adding an extra combat phase and main phase is not yet supported"
    ),
    faces = &[face!(
        name = "Relentless Assault",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::UntapAll {
        filter: &ATTACKED_CREATURE,
    }])],
);
