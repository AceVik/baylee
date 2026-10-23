#[allow(clippy::wildcard_imports)]
use super::*;

fn fecundity() -> CardIndex {
    card_index("ffa64ac6-fe55-48b8-b015-849982cd7ad8")
}

#[test]
fn fecundity_asks_the_dying_creatures_controller_and_allows_declining() {
    for (seat, accept) in [(0, true), (0, false), (1, true), (1, false)] {
        let p0 = PlayerId::new(0);
        let dying_controller = PlayerId::new(seat);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[fecundity(), grizzly_bears(), sol_ring()])
            .battlefield(1, &[grizzly_bears()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let hand_before = engine
            .state()
            .zones
            .list(ZoneLocation::Hand(dying_controller))
            .len();
        let other = PlayerId::new(1 - seat);
        let others_hand = engine.state().zones.list(ZoneLocation::Hand(other)).len();
        let bear = on_battlefield(&engine, dying_controller, grizzly_bears()).unwrap();
        crate::sba::destroy(engine.dev_state_mut(p0).unwrap(), bear);
        engine.apply(p0, PlayerAction::PassPriority).unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
        });
        let Pending::YesNo { player, .. } = engine.pending() else {
            unreachable!()
        };
        assert_eq!(*player, dying_controller);
        engine
            .apply(dying_controller, PlayerAction::YesNo(accept))
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(dying_controller))
                .len(),
            hand_before + usize::from(accept)
        );
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Hand(other)).len(),
            others_hand
        );
        let ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
        kill(&mut engine, ring);
        assert_eq!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(dying_controller))
                .len(),
            hand_before + usize::from(accept),
            "noncreature death does not trigger"
        );
    }
}
