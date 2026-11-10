//! `cards/creatures/mv_1/goblin_soothsayer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, MONS_S_GOBLIN_RAIDERS};

const P0: PlayerId = PlayerId::new(0);

fn goblin_soothsayer() -> CardIndex {
    card_index("a9277df9-1417-45bc-bebf-d9a7595f54c9")
}

/// Goblin Soothsayer (a9277df9-1417-45bc-bebf-d9a7595f54c9) — {R} 1/1:
/// "{R}, {T}, Sacrifice a Goblin: Red creatures get +1/+1 until end of turn."
#[test]
fn goblin_soothsayer_sacrifices_goblin_to_pump_red_creatures() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                goblin_soothsayer(),
                MONS_S_GOBLIN_RAIDERS,
                HILL_GIANT,
                GRIZZLY_BEARS,
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let soothsayer = on_battlefield(&engine, P0, goblin_soothsayer()).unwrap();
    let fodder = on_battlefield(&engine, P0, MONS_S_GOBLIN_RAIDERS).unwrap();
    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, goblin_soothsayer(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let sc = engine.state().object(soothsayer).unwrap().characteristics();
    assert_eq!((sc.power, sc.toughness), (Some(2), Some(2)));
    let gc = engine.state().object(giant).unwrap().characteristics();
    assert_eq!((gc.power, gc.toughness), (Some(4), Some(4)));
    let bc = engine.state().object(bears).unwrap().characteristics();
    assert_eq!((bc.power, bc.toughness), (Some(2), Some(2)));
}

/// "Red creatures": all of them, the opponent's included (a red Hill Giant is
/// 4/4), and "until end of turn": it is a 3/3 again on the opponent's turn.
#[test]
fn goblin_soothsayer_pumps_the_opponents_red_creatures_until_end_of_turn() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_soothsayer(), MONS_S_GOBLIN_RAIDERS, mountain()])
        .battlefield(1, &[HILL_GIANT, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let fodder = on_battlefield(&engine, P0, MONS_S_GOBLIN_RAIDERS).unwrap();
    let giant = on_battlefield(&engine, PlayerId::new(1), HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, PlayerId::new(1), GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, goblin_soothsayer(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (4, 4));
    assert_eq!(pt(&engine, bears), (2, 2));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(pt(&engine, giant), (3, 3));
}
