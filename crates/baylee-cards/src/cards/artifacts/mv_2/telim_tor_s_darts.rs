//! Telim'Tor's Darts — {2} — Artifact
//! Oracle: {2}, {T}: This artifact deals 1 damage to target player or planeswalker.
//! Set: MIR #321 — Mirage | Scryfall ID: 59733b76-3b44-4543-8bb2-a160232cee27 | Oracle ID: 0c978da4-9caa-4b4b-be52-889677962ec9
// IMPLEMENTED — {2}, {T}: deals 1 damage to target player or planeswalker.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TELIM_TOR_S_DARTS,
    oracle_id = "0c978da4-9caa-4b4b-be52-889677962ec9",
    scryfall_id = "59733b76-3b44-4543-8bb2-a160232cee27",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Telim'Tor's Darts",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
        }],
        target = Some(TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)),
    )],
);
