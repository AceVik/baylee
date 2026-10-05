//! Whirling Dervish — {G}{G} — Creature — Human Monk
//! Oracle: Protection from black
//! Oracle: At the beginning of each end step, if this creature dealt damage to an opponent this turn, put a +1/+1 counter on it.
//! Set: TSB #90 — Time Spiral Timeshifted | Scryfall ID: 9f1772bc-3330-4a10-9524-5bdcbd50c871 | Oracle ID: 43f89524-4b22-4707-9aba-6eb4c01d75dc
// PARTIAL — protection from black is built; the end-step counter is off the
// card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHIRLING_DERVISH,
    oracle_id = "43f89524-4b22-4707-9aba-6eb4c01d75dc",
    scryfall_id = "9f1772bc-3330-4a10-9524-5bdcbd50c871",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Whirling Dervish",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::MONK],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "no Condition or Filter says \"this creature dealt damage to an \
         opponent this turn\", so the printed intervening if has no variant \
         it can carry"
    ),
    // NOT SUPPORTED: "At the beginning of each end step, if this creature
    // dealt damage to an opponent this turn, put a +1/+1 counter on it." —
    // the end-step trigger (`Trigger::StepBegin { step: End,
    // whose: EachPlayer }`) and the counter (`Effect::AddCounter`) are
    // writable, but its intervening `if` is not: the engine's per-turn
    // record counts damage dealt to each player with no attribution to the
    // source, no `Condition` reads it, and `Trigger::DealsDamageToOpponent`
    // cannot stand in because the counter is put on at the end step rather
    // than as the damage lands.
    abilities = &[static_ability!(
        Filter::This,
        Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Black])))
    )],
);
