//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn desert_targets_self_opponents_or_planeswalkers_but_never_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for walker_target in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .hand(0, &[sunscorched_desert()])
            .battlefield(1, &[index::JACE_THE_MIND_SCULPTOR, index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let walker = on_battlefield(&e, p1, index::JACE_THE_MIND_SCULPTOR).unwrap();
        let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
        let loyalty = e
            .state()
            .object(walker)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty);
        let desert = play_land(&mut e, p0, sunscorched_desert());
        let Pending::ChooseTargets {
            options,
            player_options,
            ..
        } = e.pending()
        else {
            panic!("target")
        };
        assert_eq!(options, &vec![walker]);
        assert_eq!(player_options, &vec![p0, p1]);
        let before = e.state().snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bear],
                    players: vec![]
                }
            )
            .is_err()
        );
        assert_eq!(before, e.state().snapshot_hash());
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: if walker_target { vec![walker] } else { vec![] },
                players: if walker_target { vec![] } else { vec![p0] },
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().players[0].life,
            if walker_target { 20 } else { 19 }
        );
        assert_eq!(
            e.state()
                .object(walker)
                .unwrap()
                .counters
                .get(CounterKind::Loyalty),
            loyalty - u16::from(walker_target)
        );
        activate(&mut e, p0, sunscorched_desert(), 0);
        assert_eq!(
            e.state().players[0]
                .mana_pool
                .available(ManaColor::Colorless),
            1
        );
        assert!(is_tapped(&e, desert));
    }
}
