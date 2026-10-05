//! Singing Tree — {3}{G} — Creature — Plant
//! Oracle: {T}: Target attacking creature has base power 0 until end of turn.
//! Set: ME1 #130 — Masters Edition | Scryfall ID: 78b11d04-aee6-48c8-b4e2-2949879a30c8 | Oracle ID: 23f9bee4-ac7e-4828-82c0-576fee0d29b7
// PARTIAL — the base-power ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SINGING_TREE,
    oracle_id = "23f9bee4-ac7e-4828-82c0-576fee0d29b7",
    scryfall_id = "78b11d04-aee6-48c8-b4e2-2949879a30c8",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Singing Tree",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PLANT],
        power = Some(0),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "no Modifier sets base power alone: SetPT writes power and toughness together, and no Amount reads the target's current toughness"
    ),
    // NOT SUPPORTED: "{T}: Target attacking creature has base power 0 until end
    // of turn." — the target and the duration are sayable
    // (`TargetSpec::Object(&Filter::ATTACKING_CREATURE)` and
    // `Effect::continuous`), but the value is not: every base-P/T setter
    // (`Modifier::SetPT`, `Modifier::SetPTToCount`, `Effect::SetPTFilter`)
    // writes power and toughness together, and writing 0 beside a toughness
    // would have to invent the one the printed sentence leaves alone — no
    // `Amount` reads the target's current toughness (`Amount::TargetPower`
    // reads power). A pump cannot say it either: base power (layer 7b) and a
    // pump (7c) interact differently with counters and other pumps. The
    // ability comes off the card rather than changing a characteristic the
    // card does not touch.
);
