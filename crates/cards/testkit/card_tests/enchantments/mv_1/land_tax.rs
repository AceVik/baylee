//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn land_tax_checks_an_opponents_lands_again_on_resolution_and_searches_up_to_three_basics() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for reduce in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::LAND_TAX, plains()])
            .battlefield(1, &[forest(), forest()])
            .start();
        keep_mulligans(&mut e);
        pass_until(&mut e, |e| {
            e.state().turn.active == p0 && !stack_is_empty(e)
        });
        if reduce {
            let land = on_battlefield(&e, p1, forest()).unwrap();
            bury(&mut e, &[land]);
            pass_until(&mut e, stack_is_empty);
            assert!(matches!(e.pending(), Pending::Priority { .. }));
        } else {
            pass_until(&mut e, |e| {
                matches!(
                    e.pending(),
                    Pending::YesNo {
                        prompt: YesNoPrompt::MayDo,
                        ..
                    }
                )
            });
            e.apply(p0, PlayerAction::YesNo(true)).unwrap();
            let Pending::ChooseCards {
                options, min, max, ..
            } = e.pending()
            else {
                panic!("search: {:?}", e.pending())
            };
            assert_eq!((*min, *max), (0, 3));
            let chosen = options[..3].to_vec();
            e.apply(p0, PlayerAction::ChooseObjects { objects: chosen })
                .unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 3);
        }
    }
}
#[test]
fn land_tax_does_not_sum_opponents_land_counts() {
    let mut e = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[index::LAND_TAX, plains(), plains()])
        .battlefield(1, &[forest(), forest()])
        .battlefield(2, &[forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, PlayerId::new(0));
    assert_eq!(
        e.state()
            .zones
            .list(ZoneLocation::Hand(PlayerId::new(0)))
            .len(),
        1,
        "only the normal draw in a multiplayer game"
    );
}
