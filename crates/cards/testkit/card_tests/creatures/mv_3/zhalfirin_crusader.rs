//! `cards/creatures/mv_3/zhalfirin_crusader.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn zhalfirin_crusader() -> CardIndex {
    card_index("9e60c102-412e-4956-a7bb-a2cd737d6692")
}

/// P0 attacks P1 with their Crusader on their second turn; `blocker` (P1's)
/// blocks it. Returns once combat damage has been dealt.
fn attack_and_block(engine: &mut Engine<RegistryLookup>, attacker: ObjectId, blocker: ObjectId) {
    reach_their_main_phase(engine, P1);
    reach_their_main_phase(engine, P0);
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();
}

/// `Zhalfirin Crusader` ({1}{W}{W}, 2/2 Creature — Human Knight,
/// `Coverage::Partial`) has flanking, giving a non-flanking blocker -1/-1 until
/// end of turn.
#[test]
fn zhalfirin_crusader_has_flanking_and_shrinks_non_flanking_blocker() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[zhalfirin_crusader()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let blocker = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, zhalfirin_crusader());
    pass_until(&mut engine, stack_is_empty);
    let crusader = on_battlefield(&engine, P0, zhalfirin_crusader()).unwrap();

    attack_and_block(&mut engine, crusader, blocker);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, blocker), (1, 1));
}

/// A blocker that has flanking itself ("without flanking") is not shrunk.
#[test]
fn zhalfirin_crusader_does_not_shrink_a_blocker_with_flanking() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[zhalfirin_crusader()])
        .hand(0, &[zhalfirin_crusader()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let blocker = on_battlefield(&engine, P1, zhalfirin_crusader()).unwrap();
    cast_from_hand(&mut engine, P0, zhalfirin_crusader());
    pass_until(&mut engine, stack_is_empty);
    let crusader = on_battlefield(&engine, P0, zhalfirin_crusader()).unwrap();

    attack_and_block(&mut engine, crusader, blocker);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, blocker),
        (2, 2),
        "a flanking blocker keeps its size"
    );
}

/// P0 with a resolved Crusader and two spare Plains floating (a Crusader costs
/// three of the five), P1 with a Lightning Bolt and a Mountain.
fn board() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[mountain()])
        .hand(0, &[zhalfirin_crusader()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, zhalfirin_crusader());
    pass_until(&mut engine, stack_is_empty);
    let crusader = on_battlefield(&engine, P0, zhalfirin_crusader()).unwrap();
    (engine, crusader)
}

/// Pays {1}{W} and names `who` as the target of the redirection.
fn redirect_to(engine: &mut Engine<RegistryLookup>, who: PlayerId) {
    activate(engine, P0, zhalfirin_crusader(), 0);
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, .. } => {
            engine
                .apply(player, PlayerAction::ChoosePlayer(who))
                .unwrap();
        }
        Pending::ChooseTargets { player, .. } => {
            engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![who],
                    },
                )
                .unwrap();
        }
        pending => panic!("expected a target choice for the redirection, got {pending:?}"),
    }
    pass_until(engine, stack_is_empty);
}

/// P1 bolts the Crusader and the damage resolves, answering the redirection's
/// allocation the way its controller would (one point to the shield).
fn bolt_the_crusader(engine: &mut Engine<RegistryLookup>, crusader: ObjectId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    cast_from_hand(engine, P1, lightning_bolt());
    aim_at(engine, P1, crusader);
    for _ in 0..40 {
        if stack_is_empty(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseDamageEffect {
                player,
                choice,
                options,
                ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseDamageEffect {
                            choice,
                            effect: options[0].id,
                        },
                    )
                    .unwrap();
            }
            Pending::AllocatePrevention {
                player,
                choice,
                damage,
                total,
                ..
            } => {
                assert_eq!(player, P0, "the Crusader's controller allocates");
                assert_eq!(total, 1, "the shield redirects exactly one point");
                assert_eq!(damage.len(), 1, "one Bolt damage event");
                engine
                    .apply(
                        player,
                        PlayerAction::AllocatePrevention {
                            choice,
                            allocation: vec![(damage[0].id, 1)],
                        },
                    )
                    .unwrap();
            }
            pending => panic!("unexpected damage choice: {pending:?}"),
        }
    }
    panic!("the Bolt did not finish");
}

/// "{1}{W}: The next 1 damage that would be dealt to this creature this turn
/// is dealt to [a target] instead." Aimed at the opponent: of a Bolt's three
/// damage one is dealt to the opponent, and the Crusader (2/2) takes the other
/// two and dies. The caster loses nothing.
#[test]
fn zhalfirin_crusader_redirects_the_next_damage_to_the_chosen_opponent() {
    let (mut engine, crusader) = board();
    redirect_to(&mut engine, P1);
    bolt_the_crusader(&mut engine, crusader);

    assert_eq!(engine.state().players[1].life, 19, "one point went to P1");
    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_graveyard(&engine, P0, zhalfirin_crusader()).is_some());
}

/// The recipient is the target the activation named, not always the opponent:
/// aimed at oneself, the caster takes the point.
#[test]
fn zhalfirin_crusader_redirects_to_whichever_player_was_targeted() {
    let (mut engine, crusader) = board();
    redirect_to(&mut engine, P0);
    bolt_the_crusader(&mut engine, crusader);

    assert_eq!(engine.state().players[0].life, 19, "one point went to P0");
    assert_eq!(engine.state().players[1].life, 20);
}

/// Without the activation nothing is redirected.
#[test]
fn zhalfirin_crusader_redirects_nothing_unless_activated() {
    let (mut engine, crusader) = board();
    bolt_the_crusader(&mut engine, crusader);

    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(in_graveyard(&engine, P0, zhalfirin_crusader()).is_some());
}

/// "This turn": the shield is gone by the next turn, so a Bolt then is not
/// redirected.
#[test]
fn zhalfirin_crusader_shield_does_not_outlast_the_turn() {
    let (mut engine, crusader) = board();
    redirect_to(&mut engine, P1);
    reach_their_main_phase(&mut engine, P1);
    bolt_the_crusader(&mut engine, crusader);

    assert_eq!(
        engine.state().players[1].life,
        20,
        "the shield ended with the turn"
    );
    assert_eq!(engine.state().players[0].life, 20);
}
