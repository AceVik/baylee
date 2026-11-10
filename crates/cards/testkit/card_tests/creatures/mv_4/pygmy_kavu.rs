//! `cards/creatures/mv_4/pygmy_kavu.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Pygmy Kavu — {3}{G} Creature — Kavu 1/2
/// "When this creature enters, draw a card for each black creature your opponents control."
#[test]
fn pygmy_kavu_draws_a_card_for_each_black_creature_opponents_control() {
    let kavu = card_index("17407b73-0dbe-4a71-84ec-3aaca9651f5b");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                index::DRUDGE_SKELETONS,
            ],
        )
        .battlefield(
            1,
            &[index::DRUDGE_SKELETONS, index::BOG_WRAITH, grizzly_bears()],
        )
        .hand(0, &[kavu])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, kavu);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, kavu).is_some());
    // Cast 1 card (-1) and drew 2 cards (+2) for the two black creatures P1 controls.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
