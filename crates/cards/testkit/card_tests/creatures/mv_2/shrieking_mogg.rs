//! `cards/creatures/mv_2/shrieking_mogg.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn shrieking_mogg_etb_taps_all_other_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[card_index("088a63d4-22d2-4061-8aef-2aa3fc348639")])
        .battlefield(0, &[mountain(), mountain(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).expect("Hill Giant");

    cast_from_hand(
        &mut engine,
        p0,
        card_index("088a63d4-22d2-4061-8aef-2aa3fc348639"),
    );
    pass_until(&mut engine, stack_is_empty);

    let mogg = on_battlefield(
        &engine,
        p0,
        card_index("088a63d4-22d2-4061-8aef-2aa3fc348639"),
    )
    .expect("Shrieking Mogg");
    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "other friendly creature tapped"
    );
    assert!(
        engine
            .state()
            .object(giant)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "opponent creature tapped"
    );
    assert!(
        !engine
            .state()
            .object(mogg)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Shrieking Mogg itself is not tapped"
    );
    assert!(keywords(&engine, mogg).contains(KeywordSet::HASTE));
}
