//! `cards/sorceries/mv_4/forbidden_ritual.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn forbidden_ritual() -> CardIndex {
    card_index("4b5d9c76-8be8-4e25-b978-9750df3d8c15")
}

/// `Forbidden Ritual` ({2}{B}{B}, Sorcery, `Coverage::Partial`): sacrifices a
/// nontoken permanent you control and causes target opponent to lose 2 life.
#[test]
fn forbidden_ritual_sacrifices_nontoken_permanent_and_target_opponent_loses_two_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), GRIZZLY_BEARS])
        .hand(0, &[forbidden_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, forbidden_ritual());
    match engine.pending().clone() {
        Pending::ChoosePlayer { .. } => {
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        Pending::ChooseTargets { .. } => {
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .unwrap();
        }
        other => panic!("expected target opponent choice, got {other:?}"),
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[1].life, 18);
}
