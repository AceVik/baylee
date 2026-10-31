//! Myr Incubator exiles any number of artifacts and creates that many Myr.

use super::*;
use baylee_core::generated::subtypes;

fn myr_incubator() -> CardIndex {
    card_index("d0a493af-551e-4475-af5d-ec4e080728cb")
}

#[test]
fn myr_incubator_creates_one_myr_per_chosen_exiled_artifact() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, quiet_artifact())
        .battlefield(
            0,
            &[
                myr_incubator(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, myr_incubator(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, min, .. } = e.pending().clone() else {
        panic!("expected a library search, got {:?}", e.pending())
    };
    assert_eq!(min, 0, "any number includes zero");
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: options.into_iter().take(2).collect(),
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p0, myr_incubator()).is_some());
    assert_eq!(e.state().zones.list(ZoneLocation::Exile(p0)).len(), 2);
    let myr = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|object| {
                object.controller == p0
                    && object
                        .characteristics()
                        .subtypes
                        .contains(subtypes::creature::MYR)
            })
        })
        .count();
    assert_eq!(myr, 2);
}
