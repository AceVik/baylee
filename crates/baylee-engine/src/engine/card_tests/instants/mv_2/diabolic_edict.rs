//! `cards/instants/mv_2/diabolic_edict.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn diabolic_edict() -> CardIndex {
    card_index("058917c1-21ab-488a-9f9c-591c55f3c596")
}

/// Diabolic Edict (`058917c1-21ab-488a-9f9c-591c55f3c596`) — {1}{B} Instant:
/// "Target player sacrifices a creature of their choice."
#[test]
fn diabolic_edict_makes_target_player_sacrifice_a_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .hand(0, &[diabolic_edict()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, diabolic_edict());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_some());
}

#[test]
fn diabolic_edict_can_target_you_but_only_you_choose_your_sacrifice() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[diabolic_edict()],
        &[],
    );
    let own = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    let opposing = lk::obj(&e, P1, index::HILL_GIANT);
    lk::float(&mut e, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut e, P0, diabolic_edict());
    e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending()
    else {
        panic!("sacrifice")
    };
    assert_eq!(*player, P0);
    assert!(options.contains(&own));
    assert!(!options.contains(&opposing));
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![own] })
        .unwrap();
    lk::settle(&mut e);
    assert!(in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P1, index::HILL_GIANT).is_some());
}
