//! Balance — {1}{W} — Sorcery
//! Oracle: Each player chooses a number of lands they control equal to the number of lands controlled by the player who controls the fewest, then sacrifices the rest. Players discard cards and sacrifice creatures the same way.
//! Set: EMA #2 — Eternal Masters | Scryfall ID: ce648aa3-098b-4af0-a433-fd290bc85904 | Oracle ID: 17fa98cd-ed8f-483f-9525-7e989a82ebb2
// IMPLEMENTED — staged APNAP choices, followed by simultaneous zone changes.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BALANCE,
    oracle_id = "17fa98cd-ed8f-483f-9525-7e989a82ebb2",
    scryfall_id = "ce648aa3-098b-4af0-a433-fd290bc85904",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Balance",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::EqualizePermanents {
            filter: &Filter::LAND
        },
        Effect::EqualizeHands,
        Effect::EqualizePermanents {
            filter: &Filter::CREATURE
        },
    ])],
);
