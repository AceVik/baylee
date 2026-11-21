//! `cards/creatures/mv_3/chlorophant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn chlorophant() -> CardIndex {
    card_index("ad80d9a8-187f-4ecf-94be-e23ac3a57179")
}

/// Chlorophant ({G}{G}{G} 1/1 Elemental):
/// "At the beginning of your upkeep, you may put a +1/+1 counter on this creature.
/// Threshold — As long as there are seven or more cards in your graveyard, this
/// creature has 'At the beginning of your upkeep, you may put another +1/+1
/// counter on this creature.'"
#[test]
fn chlorophant_grows_by_up_to_one_counter_below_threshold_and_two_with_threshold() {
    let mut upkeep_prompts = 0u32;
    let mut board = vec![chlorophant()];
    board.extend_from_slice(&[plains(); 7]);
    let mut e = lk::game_with(SEED, &board, &[], &[], &[], |e| {
        upkeep_prompts += 1;
        lk::yes_no(e, true);
    });
    let elem = lk::obj(&e, lk::P0, chlorophant());
    assert_eq!(
        upkeep_prompts, 1,
        "0 cards in graveyard -> 1 upkeep trigger"
    );
    assert_eq!(
        e.state()
            .object(elem)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(pt(&e, elem), (2, 2));

    // Put 6 Plains into graveyard (< 7): next P0 upkeep still asks only once, and declining adds 0.
    let all_plains = all_on_battlefield(&e, lk::P0, plains());
    for &p in &all_plains[..6] {
        e.dev_state_mut(lk::P0)
            .unwrap()
            .move_object(
                p,
                ZoneLocation::Graveyard(lk::P0),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
    }
    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut e, false);
    reach_main_phase(&mut e, lk::P0);
    assert_eq!(
        e.state()
            .object(elem)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );

    // Move 7th Plains into graveyard -> Threshold grants a second upkeep trigger!
    e.dev_state_mut(lk::P0)
        .unwrap()
        .move_object(
            all_plains[6],
            ZoneLocation::Graveyard(lk::P0),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();
    reach_their_main_phase(&mut e, lk::P1);
    let mut threshold_prompts = 0u32;
    while !(e.state().turn.phase == Phase::FirstMain && e.state().turn.active == lk::P0) {
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                e.apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::YesNo { .. } => {
                threshold_prompts += 1;
                lk::yes_no(&mut e, true);
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert_eq!(
        threshold_prompts, 2,
        "Threshold adds a second upkeep MayDo trigger"
    );
    assert_eq!(
        e.state()
            .object(elem)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3
    );
    assert_eq!(pt(&e, elem), (4, 4));
}
