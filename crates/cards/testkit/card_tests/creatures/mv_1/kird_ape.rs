//! `cards/creatures/mv_1/kird_ape.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kird Ape — `{R}` 1/1 Ape: "This creature gets +1/+2 as long as you
/// control a Forest." (CR 604.1: the static applies exactly while its
/// condition holds, and "you" is the Ape's controller.)
///
/// Three boards. No Forest at all, the *opponent's* Forest, and its own
/// Forest: the middle one is what separates "you control a Forest" from "a
/// Forest is on the battlefield", and only the third may show the +1/+2.
#[test]
fn kird_ape_is_a_two_three_only_while_its_controller_controls_a_forest() {
    let p0 = PlayerId::new(0);

    let mut bare = Duel::new(SEED, mountain())
        .battlefield(0, &[kird_ape(), mountain()])
        .start();
    keep_mulligans(&mut bare);
    let ape = on_battlefield(&bare, p0, kird_ape()).expect("seated");
    assert_eq!(pt(&bare, ape), (1, 1), "no Forest: the printed body");

    let mut their_forest = Duel::new(SEED, mountain())
        .battlefield(0, &[kird_ape(), mountain()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut their_forest);
    let ape = on_battlefield(&their_forest, p0, kird_ape()).expect("seated");
    assert_eq!(
        pt(&their_forest, ape),
        (1, 1),
        "the opponent's Forest is not \"you control\""
    );

    let mut own_forest = Duel::new(SEED, mountain())
        .battlefield(0, &[kird_ape(), mountain(), forest()])
        .start();
    keep_mulligans(&mut own_forest);
    let ape = on_battlefield(&own_forest, p0, kird_ape()).expect("seated");
    assert_eq!(
        pt(&own_forest, ape),
        (2, 3),
        "+1/+2 while its own controller controls a Forest"
    );
}
