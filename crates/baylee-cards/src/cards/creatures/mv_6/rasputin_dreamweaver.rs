//! Rasputin Dreamweaver — {4}{W}{U} — Legendary Creature — Human Wizard
//! Oracle: Rasputin enters with seven dream counters on it.
//! Oracle: Remove a dream counter from Rasputin: Add {C}.
//! Oracle: Remove a dream counter from Rasputin: Prevent the next 1 damage that would be dealt to Rasputin this turn.
//! Oracle: At the beginning of your upkeep, if Rasputin started the turn untapped, put a dream counter on it.
//! Oracle: Rasputin can't have more than seven dream counters on it.
//! Set: ME3 #170 — Masters Edition III | Scryfall ID: 78418809-f048-4611-88cb-369f427d9c44 | Oracle ID: fef452ee-9018-4175-bb79-476544e48433
// PARTIAL — every sentence is off the card: dream counters have no assigned
// id, no modifier caps a counter count, and no condition reads whether the
// source started the turn untapped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RASPUTIN_DREAMWEAVER,
    oracle_id = "fef452ee-9018-4175-bb79-476544e48433",
    scryfall_id = "78418809-f048-4611-88cb-369f427d9c44",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Rasputin Dreamweaver",
        mana_cost = mana!("{4}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(4),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "dream counters have no assigned CounterKind id (`baylee_cards_dsl::counters` \
         has no DREAM and a card file may not spell a raw custom-counter id), no \
         Modifier caps how many counters a permanent may hold, and no Condition \
         reads \"started the turn untapped\", so all five sentences are off",
    ),
    // NOT SUPPORTED: "Rasputin enters with seven dream counters on it." —
    // `EnterModifier::WithCounters` takes a `CounterKind`, and no dream
    // counter id is assigned in `baylee_cards_dsl::counters`.
    // NOT SUPPORTED: "Remove a dream counter from Rasputin: Add {C}." —
    // `cost!(RemoveCounterSelf { kind, n: 1 })` is the shape, but there is no
    // dream counter kind to remove.
    // NOT SUPPORTED: "Remove a dream counter from Rasputin: Prevent the next
    // 1 damage that would be dealt to Rasputin this turn." — the
    // `Effect::PreventNextDamage` shield is sayable, the dream counter cost is
    // not.
    // NOT SUPPORTED: "At the beginning of your upkeep, if Rasputin started the
    // turn untapped, put a dream counter on it." — dream counters as above,
    // and no `Condition` says "started the turn untapped" (`Filter::Tapped`
    // reads the permanent now, not its history).
    // NOT SUPPORTED: "Rasputin can't have more than seven dream counters on
    // it." — no `Modifier` caps a counter count, dream counters or otherwise.
);
