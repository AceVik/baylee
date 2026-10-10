//! Printed rules exercised through the real game driver.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn fomori_vault_discards_then_counts_artifacts_at_resolution() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                fomori_vault(),
                forest(),
                forest(),
                forest(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[plains()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let vault = on_battlefield(&e, p0, fomori_vault()).unwrap();
    let discarded = in_hand(&e, p0, plains()).unwrap();
    tap_all_mana_but(&mut e, p0, Some(fomori_vault()));
    activate(&mut e, p0, fomori_vault(), 1);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discarded],
        },
    )
    .unwrap();
    assert!(is_tapped(&e, vault));
    assert_eq!(e.state().object(discarded).unwrap().zone, Zone::Graveyard);
    let before = library_size(&e, p0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((options.len(), min, max), (2, 1, 1));
    let kept = options[1];
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![kept],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(kept).unwrap().zone, Zone::Hand);
    assert_eq!(library_size(&e, p0), before - 1);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(p0)).first(),
        Some(&options[0])
    );
}
