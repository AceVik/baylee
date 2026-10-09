//! An effect that removes a creature from combat (CR 506.4), and one that
//! has it block again: False Orders.
//!
//! "Remove target creature defending player controls from combat. Creatures
//! it was blocking that had become blocked by only that creature this combat
//! become unblocked. You may have it block an attacking creature of your
//! choice." A blocked creature stays blocked when its blockers leave (CR
//! 509.1h) unless an effect says it becomes unblocked, which this one does
//! only for an attacker the removed creature blocked alone.

use super::testkit::*;
use super::*;
use crate::choice::ChoicePrompt;
use crate::event::GameEvent;
use baylee_core::generated::index;
use baylee_core::ids::Defender;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

/// p0 attacks p1 with `attackers`; p1 declares `blocks`; then p0, the
/// active player, holds priority in the declare blockers step.
#[track_caller]
fn to_blocks(
    engine: &mut Engine<RegistryLookup>,
    attackers: &[ObjectId],
    blocks: &[(ObjectId, ObjectId)],
) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&a| (a, Defender::Player(P1)))
                    .collect(),
            },
        )
        .unwrap();
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { .. } => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("on the way to blocks: {other:?}"),
        }
    }
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: blocks.to_vec(),
            },
        )
        .unwrap();
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if player == P0 => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("on the way to priority after blocks: {other:?}"),
        }
    }
    assert_eq!(engine.state().turn.step, crate::turn::Step::DeclareBlockers);
}

/// Casts False Orders at `target` and lets it resolve up to its re-block
/// question, which it returns the menu of (empty when none was asked).
#[track_caller]
fn false_orders_at(engine: &mut Engine<RegistryLookup>, target: ObjectId) -> Vec<ObjectId> {
    cast_from_hand(engine, P0, index::FALSE_ORDERS);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    match engine.pending().clone() {
        Pending::ChooseCards {
            player,
            options,
            min: 0,
            max: 1,
            prompt: ChoicePrompt::BlockWith { blocker },
            ..
        } => {
            assert_eq!((player, blocker), (P0, target), "the caster names it");
            options
        }
        _ => Vec::new(),
    }
}

/// The Angel alone blocks the Bears. False Orders takes it out of combat:
/// the Bears become unblocked and deal their damage to p1. The caster has
/// the Angel block the Hill Giant instead (a block, journalled), and the
/// Giant dies to it.
#[test]
fn an_attacker_only_the_removed_creature_blocked_becomes_unblocked() {
    let mut engine = Duel::new(6101, index::MOUNTAIN)
        .battlefield(
            0,
            &[index::GRIZZLY_BEARS, index::HILL_GIANT, index::MOUNTAIN],
        )
        .hand(0, &[index::FALSE_ORDERS])
        .battlefield(1, &[index::SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let angel = on_battlefield(&engine, P1, index::SERRA_ANGEL).unwrap();
    to_blocks(&mut engine, &[bears, giant], &[(angel, bears)]);
    assert!(engine.state().combat.is_blocked(bears));

    let menu = false_orders_at(&mut engine, angel);
    assert!(!engine.state().combat.is_blocked(bears), "unblocked");
    assert!(engine.state().combat.blocked_by(angel).is_empty());
    let mut menu = menu;
    menu.sort();
    let mut both = vec![bears, giant];
    both.sort();
    assert_eq!(menu, both, "every attacker of p1's");
    let since = engine.state().journal.last_seq();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    assert!(engine.state().combat.is_blocking(angel, giant));
    assert!(engine.state().combat.is_blocked(giant));
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .filter(|e| e.seq > since)
            .any(
                |e| matches!(e.event, GameEvent::BecameBlocker { object, attacker }
                if object == angel && attacker == giant)
            ),
        "the new block is journalled"
    );

    let before = life(&engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(life(&engine, P1), before - 2, "the Bears hit p1");
    assert!(in_graveyard(&engine, P0, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, P1, index::SERRA_ANGEL).is_some());
}

/// The Angel and the Bears both block the Hill Giant. Removing the Angel
/// does not unblock the Giant: another creature had blocked it this
/// combat (CR 509.1h). Declining the re-block leaves the Angel out of
/// combat, and the Giant's damage all goes to the Bears.
#[test]
fn an_attacker_another_creature_also_blocked_stays_blocked() {
    let mut engine = Duel::new(6102, index::MOUNTAIN)
        .battlefield(0, &[index::HILL_GIANT, index::MOUNTAIN])
        .hand(0, &[index::FALSE_ORDERS])
        .battlefield(1, &[index::SERRA_ANGEL, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let angel = on_battlefield(&engine, P1, index::SERRA_ANGEL).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    to_blocks(&mut engine, &[giant], &[(angel, giant), (bears, giant)]);

    let menu = false_orders_at(&mut engine, angel);
    assert_eq!(menu, vec![giant]);
    assert!(engine.state().combat.is_blocked(giant), "still blocked");
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert!(engine.state().combat.blocked_by(angel).is_empty());

    let before = life(&engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(life(&engine, P1), before, "blocked, so no damage to p1");
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P1, index::SERRA_ANGEL).is_some());
}

/// A creature that was not blocking may be removed and then made to block:
/// the Bears, unblocked, become blocked by the Angel.
#[test]
fn a_creature_that_did_not_block_may_be_made_to_block() {
    let mut engine = Duel::new(6103, index::MOUNTAIN)
        .battlefield(0, &[index::GRIZZLY_BEARS, index::MOUNTAIN])
        .hand(0, &[index::FALSE_ORDERS])
        .battlefield(1, &[index::SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let angel = on_battlefield(&engine, P1, index::SERRA_ANGEL).unwrap();
    to_blocks(&mut engine, &[bears], &[]);
    assert!(!engine.state().combat.is_blocked(bears));
    assert_eq!(false_orders_at(&mut engine, angel), vec![bears]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    assert!(engine.state().combat.is_blocked(bears));
    let before = life(&engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(life(&engine, P1), before);
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
}

/// "Cast this spell only during the declare blockers step", asked where the
/// target alone would allow it: in the declare attackers step the Angel is
/// already a creature a defending player controls (the combat phase has
/// begun, CR 802.2), so only the timing refuses the cast. One step later it
/// is cast.
#[test]
fn false_orders_is_cast_only_during_the_declare_blockers_step() {
    let mut engine = Duel::new(6104, index::MOUNTAIN)
        .battlefield(0, &[index::GRIZZLY_BEARS, index::MOUNTAIN, index::MOUNTAIN])
        .hand(0, &[index::FALSE_ORDERS])
        .battlefield(1, &[index::SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let first = on_battlefield(&engine, P0, index::MOUNTAIN).unwrap();
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
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
        "{:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().turn.step,
        crate::turn::Step::DeclareAttackers
    );
    tap_mana_where(&mut engine, P0, |id| id == first);
    let card = in_hand(&engine, P0, index::FALSE_ORDERS).unwrap();
    assert!(
        engine.apply(P0, PlayerAction::CastSpell { card }).is_err(),
        "not in the declare attackers step, a target or not"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
        "{:?}",
        engine.pending()
    );
    // The first Mountain's red emptied with the step (CR 500.4).
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
}

/// Whose creature is "a creature defending player controls" at a table of
/// three, P0 attacking P1 alone. This engine plays the attack multiple
/// players option (CR 802.1; Free-for-All's and Team vs. Team's), under
/// which "all the attacking player's opponents are defending players during
/// the combat phase" (CR 802.2), attacked or not, and a spell that names no
/// attacking creature picks one of them by picking the creature (802.2a:
/// "the controller of the spell or ability chooses one"). So P2's creature
/// is a target as well as P1's, and P0's own is not. Only the re-block reads
/// the creature it moves against the attackers (CR 506.3e, 802.4a): nothing
/// attacks P2, so P2's creature is offered nothing to block.
#[test]
fn every_opponent_of_the_attacker_is_a_defending_player_during_combat() {
    let p2 = PlayerId::new(2);
    let mut engine = Duel::table(6105, index::MOUNTAIN, 3)
        .battlefield(
            0,
            &[index::GRIZZLY_BEARS, index::HILL_GIANT, index::MOUNTAIN],
        )
        .hand(0, &[index::FALSE_ORDERS])
        .battlefield(1, &[index::SERRA_ANGEL])
        .battlefield(2, &[index::SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let theirs = on_battlefield(&engine, P1, index::SERRA_ANGEL).unwrap();
    let bystander = on_battlefield(&engine, p2, index::SERRA_ANGEL).unwrap();
    to_blocks(&mut engine, &[bears], &[]);
    // P2 is asked for blocks too, and declares none; `to_blocks` answers P1.
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { player, .. } => engine
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .unwrap(),
            Pending::Priority { player, .. } if player == P0 => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("{other:?}"),
        }
    }
    cast_from_hand(&mut engine, P0, index::FALSE_ORDERS);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target is asked: {:?}", engine.pending());
    };
    assert!(options.contains(&theirs), "the attacked player's");
    assert!(
        options.contains(&bystander),
        "and the other opponent's (CR 802.2)"
    );
    assert!(
        !options.contains(&giant),
        "never the attacking player's own"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bystander],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !engine.state().combat.is_blocked(bears),
        "nothing attacks P2, so its creature was offered nothing to block"
    );
}
