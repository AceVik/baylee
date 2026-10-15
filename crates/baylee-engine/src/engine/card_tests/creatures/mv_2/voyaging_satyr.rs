//! Oracle checklist: {T} cost; only lands are legal targets; successful activation untaps the selected tapped land and leaves other permanents unchanged.
#[allow(clippy::wildcard_imports)]
use super::*;
fn voyaging_satyr() -> CardIndex {
    card_index("bce18f34-848e-4dd2-9a38-6084c0695f78")
}

#[test]
fn satyr_pays_its_tap_to_untap_a_target_land_only() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[voyaging_satyr(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let satyr = on_battlefield(&e, p0, voyaging_satyr()).unwrap();
    let land = on_battlefield(&e, p0, forest()).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    tap_mana_where(&mut e, p0, |id| id == land);
    assert!(is_tapped(&e, land));
    activate(&mut e, p0, voyaging_satyr(), 0);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = e.pending().clone()
    else {
        panic!("expected land target prompt: {:?}", e.pending());
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(
        options,
        vec![land],
        "the creature and the source itself are not lands"
    );
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![land],
        },
    )
    .unwrap();
    assert!(is_tapped(&e, satyr), "{{T}} was paid on activation");
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, land));
    assert!(is_tapped(&e, satyr));
    assert!(!is_tapped(&e, elf));
}
