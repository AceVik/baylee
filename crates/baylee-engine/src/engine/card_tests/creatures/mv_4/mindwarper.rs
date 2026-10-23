//! `cards/creatures/mv_4/mindwarper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mindwarper() -> CardIndex {
    card_index("efb42ccc-6fed-47f0-84b4-e2a9a42e4dfa")
}

/// Mindwarper (`efb42ccc-6fed-47f0-84b4-e2a9a42e4dfa`) — {2}{B}{B} 0/0 Creature — Spirit (Implemented):
/// "This creature enters with three +1/+1 counters on it.
/// {2}{B}, Remove a +1/+1 counter from this creature: Target player discards a card. Activate only as a sorcery."
#[test]
fn mindwarper_enters_with_three_counters_and_removes_one_to_make_target_player_discard() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[mindwarper()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mindwarper());
    pass_until(&mut engine, stack_is_empty);

    let id = on_battlefield(&engine, P0, mindwarper()).unwrap();
    assert_eq!(power_toughness(&engine, id), (3, 3));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, mindwarper(), 0);
    if matches!(engine.pending(), Pending::ChoosePlayer { .. }) {
        engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    } else if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    {
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(power_toughness(&engine, id), (2, 2));
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn ability_requires_counters_and_your_sorcery_timing() {
    let mut e = lk::game(SEED, &[], &[], &[mindwarper()], &[]);
    lk::float(&mut e, P0, &[(ManaColor::Black, 20)]);
    cast_with_floating(&mut e, P0, mindwarper());
    lk::settle(&mut e);
    let source = lk::obj(&e, P0, mindwarper());
    assert!(lk::offered(&e, mindwarper(), P0, 0));
    reach_their_main_phase(&mut e, P1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    lk::float(&mut e, P0, &[(ManaColor::Black, 20)]);
    assert!(!lk::offered(&e, mindwarper(), P0, 0));
    reach_their_main_phase(&mut e, P0);
    lk::float(&mut e, P0, &[(ManaColor::Black, 20)]);
    assert!(lk::offered(&e, mindwarper(), P0, 0));
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(source)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 0);
    e.refresh_offer();
    assert!(!lk::offered(&e, mindwarper(), P0, 0));
}
