//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn priest_produces_sacrificed_mana_value_immediately_without_the_stack() {
    let p0 = PlayerId::new(0);
    for (artifact, expected) in [(index::MIND_STONE, 2), (index::ORNITHOPTER, 0)] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::PRIEST_OF_YAWGMOTH, artifact])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let priest = on_battlefield(&e, p0, index::PRIEST_OF_YAWGMOTH).unwrap();
        let sacrifice = on_battlefield(&e, p0, artifact).unwrap();
        activate(&mut e, p0, index::PRIEST_OF_YAWGMOTH, 0);
        assert!(matches!(e.pending(), Pending::ChooseCards { .. }));
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![sacrifice],
            },
        )
        .unwrap();
        assert!(stack_is_empty(&e));
        assert!(matches!(e.pending(),Pending::Priority{player,..} if *player==p0));
        assert_eq!(
            e.state().players[0].mana_pool.available(ManaColor::Black),
            expected
        );
        assert_eq!(e.state().object(sacrifice).unwrap().zone, Zone::Graveyard);
        assert!(is_tapped(&e, priest));
        assert!(
            e.state().object(priest).unwrap().paid.is_none(),
            "no stale payment on permanent"
        );
    }
}
