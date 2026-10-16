//! `cards/creatures/artifacts/mv_3/one_eyed_scarecrow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn one_eyed_scarecrow() -> CardIndex {
    card_index("87b61c1f-18e3-446a-a732-cff40adc5e05")
}

/// One-Eyed Scarecrow — defender 2/3; "Creatures with flying your opponents
/// control get -1/-0." An opposing flier loses a point of power, an opposing
/// ground creature and the Scarecrow's own controller's flier do not.
#[test]
fn one_eyed_scarecrow_shrinks_opposing_fliers_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // Exocomp (3/1 flier) and A.I.M. Bot (2/3 flier).
    let exocomp = card_index("fb208157-67e5-45e2-bf08-7671f65cf520");
    let bot = card_index("bd269d87-5f72-4559-bf2b-270e2ebe6154");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[one_eyed_scarecrow(), bot])
        .battlefield(1, &[exocomp, quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let scarecrow = on_battlefield(&engine, p0, one_eyed_scarecrow()).expect("seated");
    assert_eq!(pt(&engine, scarecrow), (2, 3));
    assert!(keywords(&engine, scarecrow).contains(KeywordSet::DEFENDER));

    let their_flier = on_battlefield(&engine, p1, exocomp).expect("seated");
    let their_ground = on_battlefield(&engine, p1, quiet_creature()).expect("seated");
    let my_flier = on_battlefield(&engine, p0, bot).expect("seated");
    assert_eq!(pt(&engine, their_flier), (2, 1), "3/1 flier gets -1/-0");
    assert_eq!(
        pt(&engine, their_ground),
        (1, 1),
        "a ground creature is untouched"
    );
    assert_eq!(pt(&engine, my_flier), (2, 3), "and so is my own flier");

    kill(&mut engine, scarecrow);
    assert_eq!(
        pt(&engine, their_flier),
        (3, 1),
        "the static ends with the Scarecrow"
    );
}
