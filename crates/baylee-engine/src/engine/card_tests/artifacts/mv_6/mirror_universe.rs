//! `cards/artifacts/mv_6/mirror_universe.rs`, played.
//!
//! Written from the Oracle text, the Scryfall rulings and the Comprehensive
//! Rules alone (CR 701.12 "Exchange", 119.7-8, 602.2b, 602.5).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;
use baylee_cards_dsl::{Duration, Filter, Modifier, PlayerRel};

// oracle_id = "8d231d2f-5274-4b55-8262-9eb95654d183"
fn mirror_universe() -> CardIndex {
    card_index("8d231d2f-5274-4b55-8262-9eb95654d183")
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

/// Whether Mirror Universe's one printed ability is on the offer to `seat`
/// right now.
fn offered(engine: &Engine<RegistryLookup>, seat: PlayerId) -> bool {
    let Pending::Priority { player, legal } = engine.pending() else {
        return false;
    };
    *player == seat
        && legal.abilities.iter().any(|(id, _)| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == mirror_universe()))
        })
}

/// A duel (or table) in which `seat 0` owns an untapped Mirror Universe and
/// the first thing that happens is its owner's upkeep.
#[track_caller]
fn at_my_upkeep(seats: usize, lives: &[i32]) -> Engine<RegistryLookup> {
    let mut duel = Duel::table(SEED, forest(), seats).battlefield(0, &[mirror_universe()]);
    for (seat, life) in lives.iter().enumerate() {
        duel = duel.life(seat, *life);
    }
    let mut engine = duel.start();
    keep_mulligans(&mut engine);
    assert_eq!(
        engine.state().turn.active,
        P0,
        "seat 0 takes the first turn"
    );
    assert_eq!(
        engine.state().turn.step,
        Step::Upkeep,
        "which opens with its upkeep"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
        "and its controller holds priority there: {:?}",
        engine.pending()
    );
    engine
}

/// Answers the "target opponent" question as a player would, whichever
/// shape the engine asks it in, and passes priority until the stack is
/// empty.
#[track_caller]
fn resolve_aimed_at(engine: &mut Engine<RegistryLookup>, opponent: PlayerId) {
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChoosePlayer { player, options } => {
                assert!(
                    options.contains(&opponent),
                    "{opponent:?} is a legal choice: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChoosePlayer(opponent))
                    .expect("the player came out of the offer");
            }
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert!(
                    player_options.contains(&opponent),
                    "{opponent:?} is a legal target: {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![opponent],
                        },
                    )
                    .expect("the player came out of the offer");
            }
            Pending::Priority { player, .. } => {
                if stack_is_empty(engine) {
                    return;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the ability resolves: {other:?}"),
        }
    }
    panic!("the ability never finished resolving");
}

/// An effect that says seat `seat` can't lose life this turn, registered the
/// way a resolving spell registers its continuous effects (Everybody Lives!
/// cannot be used here: it also gives the players hexproof, and Mirror
/// Universe has to be able to target the opponent).
fn cannot_lose_life(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let state = engine.dev_state_mut(seat).expect("the harness may set up");
    let modifier = Modifier::CantLoseLife {
        who: PlayerRel::You,
    };
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: seat,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: Duration::UntilEndOfTurn,
        filter: crate::effects::EffectFilter::Dsl(&Filter::Any),
        modifier,
    });
    engine.refresh_offer();
}

/// "{T}, Sacrifice this artifact: Exchange life totals with target opponent."
/// You at 5, the opponent at 18: each player gains or loses what it takes to
/// equal the other's previous total (CR 701.12c), so you end at 18 and the
/// opponent at 5 -- and nobody ends anywhere else.
#[test]
fn mirror_universe_exchanges_life_totals_with_the_opponent() {
    let mut engine = at_my_upkeep(2, &[5, 18]);
    assert_eq!((life_of(&engine, P0), life_of(&engine, P1)), (5, 18));

    activate(&mut engine, P0, mirror_universe(), 0);
    resolve_aimed_at(&mut engine, P1);

    assert_eq!(
        life_of(&engine, P0),
        18,
        "I took the opponent's previous total"
    );
    assert_eq!(life_of(&engine, P1), 5, "and the opponent took mine");
    assert_eq!(
        engine.state().turn.step,
        Step::Upkeep,
        "all of it happened in the upkeep it was activated in"
    );
}

/// "Activate only during your upkeep." Priority is walked from turn 1's
/// upkeep to turn 3's, one pass at a time, and what the seat is offered at
/// each stop is written down. The ability is on the offer at the two upkeeps
/// of the controller (the positive control at both ends of the walk) and
/// nowhere else: not in the draw step or the main phase of its own turn, and
/// not in the opponent's upkeep or any other stop of the opponent's turn.
#[test]
fn mirror_universe_is_offered_only_during_its_controllers_upkeep() {
    let mut engine = at_my_upkeep(2, &[5, 18]);
    let mut seen: Vec<(PlayerId, u32, Step, bool)> = Vec::new();
    for _ in 0..400 {
        let turn = &engine.state().turn;
        if turn.number >= 3 && turn.active == P0 && turn.step == Step::Upkeep {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                if player == P0 {
                    seen.push((
                        engine.state().turn.active,
                        engine.state().turn.number,
                        engine.state().turn.step,
                        offered(&engine, P0),
                    ));
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected on the walk: {other:?}"),
        }
    }
    assert!(
        offered(&engine, P0),
        "turn three's upkeep is the controller's own again, and the ability is back: {:?}",
        engine.pending()
    );

    assert!(
        seen.iter()
            .any(|(active, _, step, on)| *active == P0 && *step == Step::Upkeep && *on),
        "positive control: the controller's own upkeep offers it: {seen:?}"
    );
    for (active, turn, step, on) in &seen {
        assert_eq!(
            *on,
            *active == P0 && *step == Step::Upkeep,
            "turn {turn}, {step:?} of {active:?}'s turn: offered only in my own upkeep"
        );
    }
    for (label, wanted_active, wanted_step) in [
        ("my own draw or main phase", P0, Step::Main),
        ("the opponent's upkeep", P1, Step::Upkeep),
        ("the opponent's main phase", P1, Step::Main),
    ] {
        assert!(
            seen.iter()
                .any(|(active, _, step, _)| *active == wanted_active && *step == wanted_step),
            "the walk really stood in {label}, with priority: {seen:?}"
        );
    }
}

/// "{T}, Sacrifice this artifact": both halves of the cost. A tapped Mirror
/// Universe is not offered even in its controller's upkeep (the untapped one
/// is, which is the control), and once the ability is activated the artifact
/// is already in the graveyard while the ability still waits on the stack
/// (CR 602.2b: costs are paid on activation).
#[test]
fn mirror_universe_costs_its_tap_and_its_own_sacrifice() {
    let mut engine = at_my_upkeep(2, &[5, 18]);
    let artifact = on_battlefield(&engine, P0, mirror_universe()).expect("it is out");
    assert!(offered(&engine, P0), "untapped, in my upkeep: offered");

    engine.dev_state_mut(P0).unwrap().set_tapped(artifact, true);
    engine.refresh_offer();
    assert!(
        !offered(&engine, P0),
        "tapped, in the same upkeep: {{T}} cannot be paid"
    );
    engine
        .dev_state_mut(P0)
        .unwrap()
        .set_tapped(artifact, false);
    engine.refresh_offer();
    assert!(offered(&engine, P0), "untapped again: offered again");

    activate(&mut engine, P0, mirror_universe(), 0);
    // The target is named (when the engine asks) and the ability goes on the
    // stack; nobody has passed yet.
    if let Pending::ChoosePlayer { player, .. } | Pending::ChooseTargets { player, .. } =
        engine.pending().clone()
    {
        let ask = engine.pending().clone();
        match ask {
            Pending::ChoosePlayer { .. } => engine
                .apply(player, PlayerAction::ChoosePlayer(P1))
                .unwrap(),
            _ => engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![P1],
                    },
                )
                .unwrap(),
        }
    }
    assert!(
        !stack_is_empty(&engine),
        "the ability is on the stack, unresolved"
    );
    assert!(
        on_battlefield(&engine, P0, mirror_universe()).is_none(),
        "the sacrifice was paid on activation, not on resolution"
    );
    assert!(
        in_graveyard(&engine, P0, mirror_universe()).is_some(),
        "a sacrificed artifact goes to its owner's graveyard"
    );
    assert_eq!(
        (life_of(&engine, P0), life_of(&engine, P1)),
        (5, 18),
        "and nothing has been exchanged yet"
    );
}

/// "Target opponent": at a three-player table the controller picks which of
/// the two opponents trades, and the one not picked is left alone. 5 / 18 /
/// 11, trade with the third seat: 11 / 18 / 5.
#[test]
fn mirror_universe_trades_with_the_chosen_opponent_only() {
    let mut engine = at_my_upkeep(3, &[5, 18, 11]);
    activate(&mut engine, P0, mirror_universe(), 0);
    resolve_aimed_at(&mut engine, P2);
    assert_eq!(life_of(&engine, P0), 11, "I took the third seat's total");
    assert_eq!(life_of(&engine, P2), 5, "and it took mine");
    assert_eq!(
        life_of(&engine, P1),
        18,
        "the opponent not named is untouched"
    );
}

/// A player who can't lose life can't make an exchange that would lower
/// their total, and then the exchange doesn't happen at all (ruling
/// 2011-01-01; CR 119.8, 701.12a, 701.12c). Four boards share one script:
///
/// - nobody protected: the totals swap (the control);
/// - the opponent (18, would drop to 5) can't lose life: no exchange, and I
///   do not gain half of it either;
/// - I (18, would drop to 5) can't lose life: no exchange;
/// - I (5, would rise to 18) can't lose life: the exchange only raises my
///   total, so it goes through -- the rule is about a *lower* total.
///
/// In every case the artifact is gone: the sacrifice is a cost.
#[test]
fn mirror_universe_does_not_exchange_when_a_player_who_cant_lose_life_would_drop() {
    /// (my life, their life, the seat that can't lose life, expected totals)
    type Board = (i32, i32, Option<PlayerId>, (i32, i32));
    let boards: [Board; 4] = [
        (5, 18, None, (18, 5)),
        (5, 18, Some(P1), (5, 18)),
        (18, 5, Some(P0), (18, 5)),
        (5, 18, Some(P0), (18, 5)),
    ];
    for (mine, theirs, protected, expected) in boards {
        let mut engine = at_my_upkeep(2, &[mine, theirs]);
        if let Some(seat) = protected {
            cannot_lose_life(&mut engine, seat);
        }
        activate(&mut engine, P0, mirror_universe(), 0);
        resolve_aimed_at(&mut engine, P1);
        assert_eq!(
            (life_of(&engine, P0), life_of(&engine, P1)),
            expected,
            "totals {mine}/{theirs}, {protected:?} can't lose life"
        );
        assert!(
            in_graveyard(&engine, P0, mirror_universe()).is_some(),
            "the artifact was sacrificed as a cost whether or not anything was exchanged \
             (totals {mine}/{theirs}, {protected:?} can't lose life)"
        );
    }
}
