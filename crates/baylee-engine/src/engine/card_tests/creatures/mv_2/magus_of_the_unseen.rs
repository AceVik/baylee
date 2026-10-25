//! `cards/creatures/mv_2/magus_of_the_unseen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::{Phase, Step};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn magus_of_the_unseen() -> CardIndex {
    card_index("d8857977-c8a5-4703-9aaa-331b17c7a713")
}

/// Magus of the Unseen (d8857977-c8a5-4703-9aaa-331b17c7a713) — {1}{U} 1/1 Creature — Human Wizard:
/// "{1}{U}, {T}: Untap target artifact an opponent controls and gain control of it until end of turn. It gains haste until end of turn. When you lose control of the artifact, tap it."
#[test]
fn magus_of_the_unseen_untaps_gains_control_of_and_grants_haste_to_an_opponents_artifact_until_eot()
{
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[magus_of_the_unseen(), sol_ring(), island(), island()])
        .battlefield(1, &[fellwar_stone()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let own_ring = on_battlefield(&engine, P0, sol_ring()).expect("own Sol Ring seated");
    let opp_stone = on_battlefield(&engine, P1, fellwar_stone()).expect("opponent Fellwar Stone");

    // Tap P1's Fellwar Stone before activating Magus of the Unseen so we can verify UntapTarget.
    engine
        .dev_state_mut(P0)
        .unwrap()
        .set_tapped(opp_stone, true);
    assert!(entered_tapped(&engine, opp_stone));

    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, magus_of_the_unseen(), 0);
    let menu = aim_at(&mut engine, P0, opp_stone);
    assert!(
        menu.contains(&opp_stone) && !menu.contains(&own_ring),
        "only artifacts an opponent controls may be targeted, not your own"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !entered_tapped(&engine, opp_stone),
        "Magus of the Unseen untaps the targeted artifact"
    );
    assert_eq!(
        engine.state().object(opp_stone).unwrap().controller,
        P0,
        "P0 gains control of the artifact until end of turn"
    );
    assert!(
        keywords(&engine, opp_stone).contains(KeywordSet::HASTE),
        "the artifact gains haste until end of turn"
    );

    // Cleanup ends the theft, then gives priority for the delayed tap trigger.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Cleanup && !stack_is_empty(e)
    });
    assert!(
        !is_tapped(&engine, opp_stone),
        "the tap trigger has not resolved"
    );
    assert_eq!(
        engine.state().object(opp_stone).unwrap().controller,
        P1,
        "control returns to P1 at end of turn"
    );
    assert!(
        !keywords(&engine, opp_stone).contains(KeywordSet::HASTE),
        "haste expires at end of turn"
    );
    lk::settle(&mut engine);
    assert!(
        is_tapped(&engine, opp_stone),
        "losing control taps the artifact"
    );
}

fn borrowed_stone(hand: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = lk::game(
        SEED,
        &[magus_of_the_unseen()],
        &[fellwar_stone(), magus_of_the_unseen()],
        hand,
        &[],
    );
    // Let both Magi begin a turn under their controllers before using tap abilities.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    let stone = lk::obj(&engine, P1, fellwar_stone());
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    lk::use_ability(&mut engine, P0, magus_of_the_unseen(), 0, &[stone], &[]);
    assert_eq!(engine.state().object(stone).unwrap().controller, P0);
    assert!(!is_tapped(&engine, stone));
    (engine, stone)
}

#[test]
fn magus_of_the_unseen_taps_the_artifact_when_control_is_lost_before_cleanup() {
    let (mut engine, stone) = borrowed_stone(&[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(
        &mut engine,
        P1,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    activate(&mut engine, P1, magus_of_the_unseen(), 0);
    lk::aim(&mut engine, &[stone], &[]);
    pass_until(&mut engine, |e| {
        e.state().object(stone).unwrap().controller == P1 && !stack_is_empty(e)
    });
    assert_ne!(engine.state().turn.step, Step::Cleanup);
    assert!(
        !is_tapped(&engine, stone),
        "control changes before the delayed trigger resolves"
    );
    lk::settle(&mut engine);
    assert!(
        is_tapped(&engine, stone),
        "the original Magus trigger taps the stolen artifact"
    );
}

#[test]
fn magus_of_the_unseen_does_not_tap_a_new_incarnation_of_the_artifact() {
    let (mut engine, stone) = borrowed_stone(&[index::FLICKER]);
    let old_version = engine.state().object(stone).unwrap().version;
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    lk::cast_at(&mut engine, P0, index::FLICKER, &[stone], &[]);
    let returned = lk::obj(&engine, P1, fellwar_stone());
    assert_ne!(
        engine.state().object(returned).unwrap().version,
        old_version
    );
    assert!(
        !is_tapped(&engine, returned),
        "the departure trigger cannot tap the returned incarnation"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End && stack_is_empty(e)
    });
    assert!(!is_tapped(&engine, returned));
}
