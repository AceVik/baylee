//! Armageddon Clock — {6} — Artifact
//! Oracle: At the beginning of your upkeep, put a doom counter on this artifact.
//! Oracle: At the beginning of your draw step, this artifact deals damage equal to the number of doom counters on it to each player.
//! Oracle: {4}: Remove a doom counter from this artifact. Any player may activate this ability but only during any upkeep step.
//! Set: ME4 #180 — Masters Edition IV | Scryfall ID: ab55bb84-03c2-4989-8db4-0d5578ea0431 | Oracle ID: 70d90ef4-0cda-405f-abf1-734fa909efa6
// PARTIAL — every sentence revolves around a doom counter, and no assigned
// `CounterKind` id names that word, so none of the three is written.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARMAGEDDON_CLOCK,
    oracle_id = "70d90ef4-0cda-405f-abf1-734fa909efa6",
    scryfall_id = "ab55bb84-03c2-4989-8db4-0d5578ea0431",
    faces = &[face!(
        name = "Armageddon Clock",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no counters::DOOM constant is assigned in baylee_cards_dsl::counters, \
         so the doom counter cannot be named and every sentence that counts it \
         is off the card; the {4} ability also prints \"any player may \
         activate\", which no activation permission offers"
    ),
    // NOT SUPPORTED: "At the beginning of your upkeep, put a doom counter on
    // this artifact." / "At the beginning of your draw step, this artifact
    // deals damage equal to the number of doom counters on it to each
    // player." / "{4}: Remove a doom counter from this artifact. Any player
    // may activate this ability but only during any upkeep step." — the
    // counter itself cannot be written: `CounterKind::Custom` ids are assigned
    // in `baylee_cards_dsl::counters`, a card file may not spell one as a
    // number, and the registry assigns no doom word. The two step triggers
    // (`Trigger::StepBegin`, `Condition::DuringStep`) and the damage
    // (`Effect::DealDamage` to `Player(PlayerRel::EachPlayer)`,
    // `Amount::CountersOnSource`) are sayable, and they are left off together
    // with the counter they read. "Any player may activate" is the second
    // gap: no activation permission offers an ability to any player but its
    // controller, as Ifh-Bíff Efreet records.
    abilities = &[],
);
