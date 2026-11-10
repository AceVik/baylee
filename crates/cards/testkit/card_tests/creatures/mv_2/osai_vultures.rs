//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn vultures_count_each_end_step_once_and_pay_real_carrion_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let kind = baylee_cards_dsl::counters::CARRION;
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::OSAI_VULTURES, index::DOUBLING_SEASON])
        .battlefield(1, &[grizzly_bears(), giant_spider()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bird = on_battlefield(&e, p0, index::OSAI_VULTURES).unwrap();
    reach_their_main_phase(&mut e, p1);
    assert_eq!(counters_on(&e, bird, kind), 0);
    let bear = on_battlefield(&e, p1, grizzly_bears()).unwrap();
    let spider = on_battlefield(&e, p1, giant_spider()).unwrap();
    bury(&mut e, &[bear, spider]);
    super::super::walk_to_own_main(&mut e, p0);
    assert_eq!(
        counters_on(&e, bird, kind),
        2,
        "one placement despite two deaths, doubled by Season"
    );
    activate(&mut e, p0, index::OSAI_VULTURES, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, bird, kind), 0);
    assert_eq!(pt(&e, bird), (2, 2));
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bird,
                ability_index: 1
            }
        )
        .is_err()
    );
    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, bird), (1, 1));
    assert_eq!(counters_on(&e, bird, kind), 0, "no deaths on the next turn");
}
