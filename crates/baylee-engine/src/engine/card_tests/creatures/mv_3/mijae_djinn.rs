//! `cards/creatures/mv_3/mijae_djinn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::event::GameEvent;

fn mijae_djinn() -> CardIndex {
    card_index("61bebdfa-5df0-4952-abf9-dc5e0e4f57ea")
}

/// Seeds whose first coin flip, in the game below, is won and lost.
const WON_SEED: u64 = 3;
const LOST_SEED: u64 = 1;

/// Every flip the journal holds, in order, as "won".
fn flips(engine: &Engine<RegistryLookup>) -> Vec<bool> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::CoinFlipped { won, .. } => Some(won),
            _ => None,
        })
        .collect()
}

/// Mijae Djinn (a 6/3, `{R}{R}{R}`) attacks an empty board on seat 0's
/// first turn; the engine rests with the attack declared and its trigger
/// resolved.
fn mijae_attacks(seed: u64) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[mijae_djinn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let djinn = on_battlefield(&engine, p0, mijae_djinn()).expect("seated");
    assert_eq!(pt(&engine, djinn), (6, 3), "the printed 6/3");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(djinn, Defender::Player(p1))],
            },
        )
        .expect("it attacks");
    pass_until(&mut engine, |e| !flips(e).is_empty() && stack_is_empty(e));
    (engine, djinn)
}

/// "Whenever this creature attacks, flip a coin. If you lose the flip,
/// remove this creature from combat and tap it." Lost: it is no longer
/// attacking, it is tapped, and the defending player takes nothing.
#[test]
fn a_lost_flip_takes_mijae_djinn_out_of_combat_and_it_deals_nothing() {
    let (mut engine, djinn) = mijae_attacks(LOST_SEED);
    assert_eq!(flips(&engine), vec![false], "exactly one flip, and lost");
    assert!(
        !engine.state().combat.is_attacking(djinn),
        "removed from combat (CR 506.4)"
    );
    assert!(is_tapped(&engine, djinn), "and tapped");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[1].life,
        20,
        "out of combat, it deals no combat damage"
    );
    assert_eq!(flips(&engine), vec![false], "the trigger fired once");
}

/// A won flip changes nothing: the Djinn stays an attacker and its 6 power
/// reaches the unblocked defending player.
#[test]
fn a_won_flip_leaves_mijae_djinn_attacking_for_six() {
    let (mut engine, djinn) = mijae_attacks(WON_SEED);
    assert_eq!(flips(&engine), vec![true], "exactly one flip, and won");
    assert!(
        engine.state().combat.is_attacking(djinn),
        "still attacking: nothing was said about a win"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(engine.state().players[1].life, 14, "six combat damage");
    assert_eq!(flips(&engine), vec![true], "the trigger fired once");
}
