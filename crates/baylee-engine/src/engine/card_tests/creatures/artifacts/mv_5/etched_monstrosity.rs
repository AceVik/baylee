//! `cards/creatures/artifacts/mv_5/etched_monstrosity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn etched_monstrosity() -> CardIndex {
    card_index("d0dd582a-ac35-4a46-beca-6304fdd2d358")
}

/// Etched Monstrosity — "enters with five -1/-1 counters on it" (a 10/10
/// becomes 5/5), and "{W}{U}{B}{R}{G}, Remove five -1/-1 counters from this
/// creature: Target player draws three cards": the counters are the price,
/// the printed 10/10 is what is left, and either player may be the target.
#[test]
fn etched_monstrosity_enters_with_counters_and_trades_them_for_three_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for drawer in [p0, p1] {
        let mut engine = Duel::new(SEED, forest())
            .hand(0, &[etched_monstrosity()])
            .battlefield(
                0,
                &[
                    plains(),
                    island(),
                    swamp(),
                    mountain(),
                    forest(),
                    forest(),
                    forest(),
                    forest(),
                    forest(),
                    forest(),
                ],
            )
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        // Five Forests pay {5}; the four colours and the sixth Forest stay.
        let forests = all_on_battlefield(&engine, p0, forest());
        assert_eq!(forests.len(), 6);
        tap_mana_where(&mut engine, p0, |id| forests[..5].contains(&id));
        cast_with_floating(&mut engine, p0, etched_monstrosity());
        pass_until(&mut engine, stack_is_empty);
        let monster = on_battlefield(&engine, p0, etched_monstrosity()).expect("resolved");
        assert_eq!(counters_on(&engine, monster, CounterKind::M1M1), 5);
        assert_eq!(
            pt(&engine, monster),
            (5, 5),
            "10/10 under five -1/-1 counters"
        );

        let hand = engine.state().zones.list(ZoneLocation::Hand(drawer)).len();
        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, etched_monstrosity(), 0);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![drawer],
                },
            )
            .expect("any player is a legal target");
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            counters_on(&engine, monster, CounterKind::M1M1),
            0,
            "the cost"
        );
        assert_eq!(pt(&engine, monster), (10, 10));
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Hand(drawer)).len(),
            hand + 3,
            "the target draws three"
        );
    }
}
