//! `cards/creatures/mv_3/ball_lightning.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ball_lightning() -> CardIndex {
    card_index("7485cf25-eb41-4397-be13-7f0b8c10c70a")
}

/// Turn one, main phase, Ball Lightning cast off three Mountains and resolved.
fn with_ball(opponent: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[ball_lightning()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, opponent)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, ball_lightning());
    pass_until(&mut engine, stack_is_empty);
    let ball = on_battlefield(&engine, P0, ball_lightning()).expect("Ball Lightning resolved");
    (engine, ball)
}

fn attack(engine: &mut Engine<RegistryLookup>, ball: ObjectId) {
    pass_until(engine, |e| e.state().turn.step == Step::DeclareAttackers);
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ball, Defender::Player(P1))],
            },
        )
        .expect("Ball Lightning attacks");
}

/// Ball Lightning (7485cf25-eb41-4397-be13-7f0b8c10c70a) — "Trample. Haste.
/// At the beginning of the end step, sacrifice this creature." It is a 6/1:
/// cast, it attacks the same turn, hits for six, and is sacrificed at the end
/// step.
#[test]
fn ball_lightning_hits_with_haste_and_is_sacrificed_at_the_end_step() {
    let (mut engine, ball) = with_ball(&[]);
    assert_eq!(pt(&engine, ball), (6, 1));
    let words = keywords(&engine, ball);
    assert!(words.contains(KeywordSet::TRAMPLE));
    assert!(words.contains(KeywordSet::HASTE));

    attack(&mut engine, ball);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        14,
        "haste: it attacked the turn it came, for six"
    );
    assert_eq!(
        engine.state().object(ball).map(|o| o.zone),
        Some(Zone::Battlefield),
        "still here after combat"
    );

    pass_until(&mut engine, |e| e.state().turn.step == Step::End);
    assert_eq!(
        engine.state().object(ball).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the trigger has not resolved yet"
    );
    assert!(!stack_is_empty(&engine), "the sacrifice is on the stack");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, ball_lightning()).is_none(),
        "sacrificed at the beginning of the end step"
    );
    assert!(in_graveyard(&engine, P0, ball_lightning()).is_some());
}

/// It is sacrificed even if it never attacks.
#[test]
fn ball_lightning_is_sacrificed_even_when_it_stays_home() {
    let (mut engine, _ball) = with_ball(&[]);
    pass_until(&mut engine, |e| e.state().turn.step == Step::End);
    assert!(!stack_is_empty(&engine), "the sacrifice is on the stack");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, ball_lightning()).is_none());
    assert!(in_graveyard(&engine, P0, ball_lightning()).is_some());
    assert_eq!(engine.state().players[1].life, 20);
}

/// Trample: blocked by a 1/1, the other five damage go through to the player.
#[test]
fn ball_lightning_tramples_over_a_blocker() {
    let (mut engine, ball) = with_ball(&[quiet_creature()]);
    let elf = on_battlefield(&engine, P1, quiet_creature()).expect("their Elf");
    attack(&mut engine, ball);
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, ball)],
            },
        )
        .expect("the Elf blocks");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        15,
        "one point to the 1/1 blocker, five over the top"
    );
    assert!(
        in_graveyard(&engine, P1, quiet_creature()).is_some(),
        "the blocker took lethal damage"
    );
}
