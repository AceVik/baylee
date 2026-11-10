//! `cards/lands/utility/valakut_the_molten_pinnacle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// The entering Mountain is excluded from the five-other-Mountains condition.
#[test]
fn valakut_does_not_trigger_on_fifth_mountain() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                valakut_the_molten_pinnacle(),
            ],
        )
        .hand(0, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = engine.state().players[1].life;
    play_land(&mut engine, p0, mountain());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, life);
    assert!(!matches!(engine.pending(), Pending::ChooseTargets { .. }));
}

#[test]
fn valakut_triggers_on_sixth_mountain_and_damages_opponent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                valakut_the_molten_pinnacle(),
            ],
        )
        .hand(0, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = engine.state().players[1].life;
    play_land(&mut engine, p0, mountain());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, life - 3);
}

/// p0 with Valakut and five Mountains, so that a sixth Mountain of theirs
/// would trigger it; a Forest in p0's hand and a Mountain in p1's.
fn valakut_with_five_mountains() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                valakut_the_molten_pinnacle(),
            ],
        )
        .hand(0, &[forest()])
        .hand(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// "Whenever a **Mountain** you control enters": with the five other Mountains
/// there, a Forest entering under p0's control is not a Mountain and asks for
/// no target and deals no damage.
#[test]
fn valakut_does_not_trigger_on_a_land_that_is_not_a_mountain() {
    let p0 = PlayerId::new(0);
    let mut engine = valakut_with_five_mountains();
    let lives = [
        engine.state().players[0].life,
        engine.state().players[1].life,
    ];
    play_land(&mut engine, p0, forest());
    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "a Forest is no Mountain: Valakut asks for no target, got {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(!matches!(engine.pending(), Pending::ChooseTargets { .. }));
    assert_eq!(engine.state().players[0].life, lives[0]);
    assert_eq!(engine.state().players[1].life, lives[1]);
}

/// "Whenever a Mountain **you control** enters": a Mountain entering under
/// the opponent's control on their turn, with p0 still holding Valakut and its
/// five Mountains, triggers nothing for p0.
#[test]
fn valakut_does_not_trigger_on_an_opponents_mountain() {
    let p1 = PlayerId::new(1);
    let mut engine = valakut_with_five_mountains();
    let lives = [
        engine.state().players[0].life,
        engine.state().players[1].life,
    ];
    reach_their_main_phase(&mut engine, p1);
    play_land(&mut engine, p1, mountain());
    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "p1's Mountain is not one p0 controls, got {:?}",
        engine.pending()
    );
    // On through p1's turn to p0's next main phase: a Valakut trigger would
    // stop the walk at its target question, which panics.
    reach_their_main_phase(&mut engine, PlayerId::new(0));
    assert_eq!(engine.state().players[0].life, lives[0]);
    assert_eq!(engine.state().players[1].life, lives[1]);
}
