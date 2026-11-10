//! `cards/creatures/artifacts/mv_5/ultron_machine_overlord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ultron_machine_overlord() -> CardIndex {
    card_index("19fd953c-b6d4-457f-b85f-b681c5b0e341")
}

fn anodet_lurker() -> CardIndex {
    card_index("5c2dbe90-5784-4bc4-a715-76a61c1ecfbc")
}

fn monoist_sentry() -> CardIndex {
    card_index("0b4cb111-ecf7-4ff7-a9e3-c944054f3370")
}

/// Ultron, Machine Overlord — flying; "Other Robots and Constructs you
/// control get +2/+2." Constructs and Robots of mine grow, he does not, and
/// neither do my other creatures or the opponent's Construct.
#[test]
fn ultron_machine_overlord_pumps_my_other_robots_and_constructs() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ultron_machine_overlord(),
                anodet_lurker(),
                monoist_sentry(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[anodet_lurker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ultron = on_battlefield(&engine, p0, ultron_machine_overlord()).expect("seated");
    let construct = on_battlefield(&engine, p0, anodet_lurker()).expect("seated");
    let robot = on_battlefield(&engine, p0, monoist_sentry()).expect("seated");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    let theirs = on_battlefield(&engine, PlayerId::new(1), anodet_lurker()).expect("seated");
    assert!(keywords(&engine, ultron).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, ultron), (4, 4), "\"other\": not himself");
    assert_eq!(
        pt(&engine, construct),
        (5, 5),
        "a Construct of mine: 3/3 + 2/+2"
    );
    assert_eq!(pt(&engine, robot), (6, 3), "a Robot of mine: 4/1 + 2/+2");
    assert_eq!(
        pt(&engine, ogre),
        (2, 2),
        "my other creatures are not Robots or Constructs"
    );
    assert_eq!(
        pt(&engine, theirs),
        (3, 3),
        "the opponent's Construct is not mine"
    );
}
