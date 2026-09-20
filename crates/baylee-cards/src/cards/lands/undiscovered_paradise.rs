//! Undiscovered Paradise — (no cost) — Land
//! Oracle: {T}: Add one mana of any color. During your next untap step, as you untap your permanents, return this land to its owner's hand.
//! Set: VIS #167 — Visions | Scryfall ID: 5f6e8830-5e62-4945-8b73-60f0628d38e7 | Oracle ID: 76c33d54-ce55-400e-bec5-79d33a5a20fb
// PARTIAL — {T}: Add one mana of any color. The second clause has no
// vocabulary: nothing triggers at an untap step and nothing returns the
// source itself to its owner's hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNDISCOVERED_PARADISE,
    oracle_id = "76c33d54-ce55-400e-bec5-79d33a5a20fb",
    scryfall_id = "5f6e8830-5e62-4945-8b73-60f0628d38e7",
    faces = &[face!(name = "Undiscovered Paradise", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "\"During your next untap step, as you untap your permanents, return this land to its owner's hand\": Trigger::StepBegin has no untap step and no effect returns the source to its owner's hand"
    ),
    abilities = &[
        // NOT SUPPORTED: "During your next untap step, as you untap your
        // permanents, return this land to its owner's hand." — a delayed
        // trigger at an untap step is not sayable (StepKind has Upkeep,
        // Draw, CombatBegin and End only), and no effect moves the source
        // permanent to its owner's hand.
        mana_ability!(&[Effect::mana_of_any_color()]),
    ],
);
