//! Cyclone — {2}{G}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, put a wind counter on this enchantment, then sacrifice this enchantment unless you pay {G} for each wind counter on it. If you pay, this enchantment deals damage equal to the number of wind counters on it to each creature and each player.
//! Set: ME4 #148 — Masters Edition IV | Scryfall ID: e5b304be-916f-46bc-8b09-cf1bbeb1872b | Oracle ID: fc9e82b2-f148-431b-9312-97b7101cffe4
// PARTIAL — the upkeep trigger is off the card: the {G}-per-counter price is
// not sayable and no wind counter id is assigned.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CYCLONE,
    oracle_id = "fc9e82b2-f148-431b-9312-97b7101cffe4",
    scryfall_id = "e5b304be-916f-46bc-8b09-cf1bbeb1872b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Cyclone",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Partial(
        "the upkeep trigger is not written: 'pay {G} for each wind counter on \
         it' has no spelling — PlayerMayPayOr charges a generic Amount and \
         PlayerMayPayManaOr one fixed ManaCost — and counters::ASSIGNED has \
         no wind counter for the effect to count"
    ),
    // NOT SUPPORTED: "At the beginning of your upkeep, put a wind counter on
    // this enchantment, then sacrifice this enchantment unless you pay {G}
    // for each wind counter on it. If you pay, this enchantment deals damage
    // equal to the number of wind counters on it to each creature and each
    // player." — the counter, the sacrifice, the counted damage and the "if
    // you pay" branch are all sayable, but the price is not: `PlayerMayPayOr`
    // is the cumulative-upkeep door and its `Amount` is paid as generic mana
    // (the printed {1} of Mystic Remora), so a colored, per-counter price has
    // no variant; `PlayerMayPayManaOr` takes one printed `ManaCost` and
    // cannot repeat it a counted number of times. The counter itself is also
    // unassigned — `counters::ASSIGNED` ends at mire and a bare
    // `CounterKind::Custom(n)` is the collision that registry exists to stop.
    abilities = &[],
);
