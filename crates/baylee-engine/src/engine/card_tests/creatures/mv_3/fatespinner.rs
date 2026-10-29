//! Fatespinner — `{1}{U}{U}` 1/2 Creature — Human Wizard (MRD).
//! Oracle: "At the beginning of each opponent's upkeep, that player chooses draw step, main phase, or combat phase. The player skips each instance of the chosen step or phase this turn."

use super::*;

fn fatespinner() -> CardIndex {
    card_index("676935c8-22e0-427f-9566-16778cea0873")
}

#[test]
fn fatespinner_makes_opponent_choose_and_skip_draw_step_on_their_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let spinner = fatespinner();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[spinner])
        .hand(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    // Choice 0 = draw step.
    engine.apply(p1, PlayerAction::ChooseNumber(0)).unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before,
        "p1 skipped their draw step this turn"
    );
}

fn fatespinner_turn(choice: u32, stop: crate::turn::Step) -> (bool, bool) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let spinner = fatespinner();
    let mut engine = Duel::new(SEED, island()).battlefield(0, &[spinner]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseNumber(choice))
        .unwrap();

    let mut saw_main = false;
    let mut saw_combat = false;
    for _ in 0..80 {
        if e_state_matches(&engine, p1, crate::turn::Step::Main) {
            saw_main = true;
        }
        if e_state_matches(&engine, p1, crate::turn::Step::CombatBegin) {
            saw_combat = true;
        }
        if e_state_matches(&engine, p1, stop) {
            return (saw_main, saw_combat);
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected Fatespinner transition: {other:?}"),
        }
    }
    panic!("Fatespinner never reached {stop:?}");
}

fn e_state_matches(
    engine: &Engine<RegistryLookup>,
    player: PlayerId,
    step: crate::turn::Step,
) -> bool {
    engine.state().turn.active == player && engine.state().turn.step == step
}

#[test]
fn fatespinner_skips_both_main_phases_but_not_combat() {
    let (saw_main, saw_combat) = fatespinner_turn(1, crate::turn::Step::End);
    assert!(!saw_main, "both main phases were skipped");
    assert!(saw_combat, "the combat phase still happened");
}

#[test]
fn fatespinner_skips_combat_but_keeps_the_main_phases() {
    let (saw_main, saw_combat) = fatespinner_turn(2, crate::turn::Step::Main);
    assert!(saw_main, "the first main phase still happened");
    assert!(!saw_combat, "the combat phase was skipped");
}
