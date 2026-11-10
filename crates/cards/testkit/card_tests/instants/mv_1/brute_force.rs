// Oracle checklist: own/opponent creature legal; noncreatures excluded; exact P/T; bystander; next-turn expiration.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("9880ba09-d5b8-4675-bfb4-2161d86d2d41")
}

#[test]
fn creature_filter_exact_change_and_cleanup() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for seat in [p0, p1] {
        let mut game = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[mountain(), mountain(), fangren_hunter(), quiet_artifact()],
            )
            .battlefield(1, &[fangren_hunter()])
            .hand(0, &[subject()])
            .start();
        keep_mulligans(&mut game);
        reach_main_phase(&mut game, p0);
        let target = on_battlefield(&game, seat, fangren_hunter()).unwrap();
        let other =
            on_battlefield(&game, if seat == p0 { p1 } else { p0 }, fangren_hunter()).unwrap();
        let artifact = on_battlefield(&game, p0, quiet_artifact()).unwrap();
        let land = on_battlefield(&game, p0, mountain()).unwrap();
        cast_from_hand(&mut game, p0, subject());
        let options = aim_at(&mut game, p0, target);
        assert!(options.contains(&other));
        assert!(!options.contains(&artifact));
        assert!(!options.contains(&land));
        pass_until(&mut game, stack_is_empty);
        assert_eq!(pt(&game, target), (7, 7));
        assert_eq!(pt(&game, other), (4, 4));

        let turn = game.state().turn.number;
        pass_until(&mut game, |e| e.state().turn.number > turn);
        assert_eq!(game.state().turn.active, p1);
        assert_eq!(pt(&game, target), (4, 4));

        assert!(in_graveyard(&game, p0, subject()).is_some());
    }
}
