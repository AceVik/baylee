//! Barbarian Ring — (no cost) — Land
//! Oracle: {T}: Add {R}. This land deals 1 damage to you.
//! Oracle: Threshold — {R}, {T}, Sacrifice this land: It deals 2 damage to any target. Activate only if there are seven or more cards in your graveyard.
//! Set: MH3 #299 — Modern Horizons 3 | Scryfall ID: c184406c-b22c-4b9b-9d3a-3e7b17efd8a0 | Oracle ID: eeb9377b-72c1-4214-9a66-0f55577c17d1
// PARTIAL — the mana ability is built: it adds {R} and deals 1 damage to you
// in the same resolution (a mana ability may carry effects beside the mana,
// as the depletion lands do). The threshold ability is not built.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BARBARIAN_RING,
    oracle_id = "eeb9377b-72c1-4214-9a66-0f55577c17d1",
    scryfall_id = "c184406c-b22c-4b9b-9d3a-3e7b17efd8a0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(name = "Barbarian Ring", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "threshold — \"Activate only if there are seven or more cards in your graveyard\" has no Condition variant (Condition reads the battlefield, the source's counters, or an opponent's graveyard), so the {R}, {T}, Sacrifice this land ability is left off rather than offered ungated on an empty graveyard",
    ),
    abilities = &[mana_ability!(&[
        Effect::mana(ManaColor::Red, 1),
        Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Player(PlayerRel::You),
        },
    ])],
);

// NOT SUPPORTED: "Threshold — {R}, {T}, Sacrifice this land: It deals 2 damage
// to any target. Activate only if there are seven or more cards in your
// graveyard." Condition has no "your graveyard holds at least N cards"
// variant — OpponentGraveyardCountAtLeast counts the wrong player's
// graveyard — and the ability's other three quarters (mana cost, tap,
// sacrifice, and 2 damage to TargetSpec::AnyTarget) would compile and run
// without the gate, which is a strictly stronger card than the printed one.
