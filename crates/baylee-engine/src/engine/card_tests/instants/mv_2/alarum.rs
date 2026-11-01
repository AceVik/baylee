//! `cards/instants/mv_2/alarum.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn alarum() -> CardIndex {
    card_index("18e96674-6fd8-4046-a3fe-7ba80138d85d")
}

#[test]
fn alarum_untaps_nonattacking_creature_and_gives_plus_one_plus_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[alarum()])
        .battlefield(0, &[GRIZZLY_BEARS, plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    engine.dev_state_mut(p0).unwrap().set_tapped(bear, true);

    cast_from_hand(&mut engine, p0, alarum());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    let bear_obj = engine.state().object(bear).unwrap();
    assert!(!bear_obj.status.contains(Status::TAPPED));
    assert_eq!(pt(&engine, bear), (3, 5));
}

#[test]
fn alarum_cannot_target_an_attacking_creature_and_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[alarum()])
        .battlefield(0, &[GRIZZLY_BEARS, HILL_GIANT, plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let enemy = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();
    cast_from_hand(&mut engine, p0, alarum());
    let options = aim_at(&mut engine, p0, giant);
    assert!(options.contains(&giant), "a creature that is not attacking");
    assert!(options.contains(&enemy), "any player's creature");
    assert!(!options.contains(&attacker), "an attacking creature");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (4, 6));
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, giant), (3, 3), "until end of turn");
}
