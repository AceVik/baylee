//! `cards/creatures/mv_4/emeritus_of_woe.rs`, played.
#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

/// "At the beginning of your end step, if two or more creatures died this
/// turn, this creature becomes prepared." It is an intervening "if" (CR 603.4):
/// with fewer than two deaths the trigger never goes on the stack, and with
/// two it re-prepares the creature.
#[test]
fn emeritus_becomes_prepared_at_the_end_step_only_after_two_creature_deaths() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for deaths in [0usize, 1, 2] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::EMERITUS_OF_WOE])
            .battlefield(1, &[grizzly_bears(), llanowar_elves()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let emeritus = on_battlefield(&e, p0, index::EMERITUS_OF_WOE).unwrap();
        let victims = [
            on_battlefield(&e, p1, grizzly_bears()).unwrap(),
            on_battlefield(&e, p1, llanowar_elves()).unwrap(),
        ];
        let prepared = |e: &Engine<RegistryLookup>| {
            e.state()
                .object(emeritus)
                .unwrap()
                .riders
                .contains(&crate::object::Rider::Prepared)
        };
        // Unprepare it (as if its tutor had been cast), then let creatures die.
        e.dev_state_mut(p0)
            .unwrap()
            .object_mut(emeritus)
            .unwrap()
            .riders
            .retain(|r| *r != crate::object::Rider::Prepared);
        assert!(!prepared(&e));
        bury(&mut e, &victims[..deaths]);
        pass_until(&mut e, |e| {
            e.state().turn.active == p0
                && e.state().turn.step == crate::turn::Step::End
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
        });
        let on_the_stack = e.state().zones.list(ZoneLocation::Stack).len();
        assert_eq!(
            on_the_stack,
            usize::from(deaths >= 2),
            "{deaths} deaths: the trigger is on the stack only when its condition holds"
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(prepared(&e), deaths >= 2, "{deaths} deaths");
    }
}
