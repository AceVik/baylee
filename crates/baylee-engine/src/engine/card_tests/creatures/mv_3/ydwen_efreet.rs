//! `cards/creatures/mv_3/ydwen_efreet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::event::GameEvent;
use baylee_cards_dsl::KeywordSet;

fn ydwen_efreet() -> CardIndex {
    card_index("15b9b0cc-47ef-4147-aba7-a7adae41921b")
}

/// Seeds whose first coin flip, in the game below, is won and lost.
const WON_SEED: u64 = 3;
const LOST_SEED: u64 = 1;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

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

/// Seat 0 attacks seat 1 with its Grizzly Bears (2/2); seat 1's Ydwen Efreet
/// (3/6) blocks it. Returns the engine once the block is declared and the
/// trigger has resolved, with the Bears and the Efreet.
fn ydwen_blocks_the_bears(seed: u64) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[grizzly_bears()])
        .battlefield(1, &[ydwen_efreet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, grizzly_bears()).expect("seated");
    let efreet = on_battlefield(&engine, P1, ydwen_efreet()).expect("seated");
    assert_eq!(pt(&engine, efreet), (3, 6), "the printed 3/6");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .expect("the Bears attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(efreet, bears)],
            },
        )
        .expect("the Efreet blocks");
    pass_until(&mut engine, |e| !flips(e).is_empty() && stack_is_empty(e));
    (engine, bears, efreet)
}

/// "Whenever this creature blocks, flip a coin. If you lose the flip, remove
/// this creature from combat and it can't block this turn. Creatures it was
/// blocking that had become blocked by only this creature this combat become
/// unblocked." Lost: the Bears were blocked by the Efreet alone, so they are
/// unblocked and hit the player; nobody is damaged in a fight that no longer
/// happens; and the Efreet carries "can't block" until the turn ends only.
#[test]
fn a_lost_flip_unblocks_what_only_ydwen_efreet_blocked() {
    let (mut engine, bears, efreet) = ydwen_blocks_the_bears(LOST_SEED);
    assert_eq!(flips(&engine), vec![false], "exactly one flip, and lost");
    assert!(
        engine.state().combat.blocked_by(efreet).is_empty(),
        "the Efreet is out of combat"
    );
    assert!(
        !engine.state().combat.is_blocked(bears),
        "the Bears were blocked by it alone, so they are unblocked again"
    );
    assert!(
        keywords(&engine, efreet).contains(KeywordSet::CANT_BLOCK),
        "it can't block this turn"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the unblocked Bears deal their 2 to the defending player"
    );
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_some(),
        "and the Efreet's 3 power never reached them"
    );
    assert!(
        keywords(&engine, efreet).contains(KeywordSet::CANT_BLOCK),
        "still true in the second main phase of that turn"
    );
    assert_eq!(flips(&engine), vec![false], "the trigger fired once");

    // "...this turn": by seat 0's next turn the restriction is gone, and the
    // Efreet blocks again (and so flips again).
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        !keywords(&engine, efreet).contains(KeywordSet::CANT_BLOCK),
        "the effect lasted until end of turn and no longer"
    );
    let bears = on_battlefield(&engine, P0, grizzly_bears()).expect("still here");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .expect("the Bears attack again");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(efreet, bears)],
            },
        )
        .expect("a fresh turn: the Efreet may block");
    assert!(engine.state().combat.is_blocking(efreet, bears));
}

/// A won flip changes nothing: the Efreet stays in front of the Bears, which
/// stay blocked, take 3 and die, and the player takes nothing.
#[test]
fn a_won_flip_leaves_ydwen_efreet_blocking() {
    let (mut engine, bears, efreet) = ydwen_blocks_the_bears(WON_SEED);
    assert_eq!(flips(&engine), vec![true], "exactly one flip, and won");
    assert!(
        engine.state().combat.is_blocking(efreet, bears),
        "still blocking"
    );
    assert!(engine.state().combat.is_blocked(bears));
    assert!(
        !keywords(&engine, efreet).contains(KeywordSet::CANT_BLOCK),
        "no restriction on a win"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(engine.state().players[1].life, 20, "nothing got through");
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_none(),
        "the Bears took 3 from the Efreet"
    );
    assert!(
        on_battlefield(&engine, P1, ydwen_efreet()).is_some(),
        "2 damage on a 3/6 is nothing"
    );
    assert_eq!(flips(&engine), vec![true], "the trigger fired once");
}
