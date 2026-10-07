//! `cards/lands/utility/kessig_wolf_run.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kessig Wolf Run: "{T}: Add {C}." / "{X}{R}{G}, {T}: Target creature gets
/// +X/+0 and gains trample until end of turn."
///
/// Written for the trample and reaching X = 0, because that is all the card
/// could do: the announcement CR 602.2b asks for did not exist, so every
/// `Amount::X` on an activated ability in this pool was a zero. It is played
/// for X = 1 now — three mana float here, the Forest, the Mountain and the
/// Elves' own {G} — and the +1/+0 is the half that would have gone unnoticed
/// forever, since a pump of nothing is indistinguishable from a pump that
/// was never applied.
///
/// The number comes **before** the target (CR 601.2b, then 601.2c), which is
/// the order this test now walks and the reason the storage lands' cost has
/// a lint of its own.
#[test]
fn kessig_wolf_run_pumps_by_the_x_it_announces() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(115, forest())
        .battlefield(
            0,
            &[kessig_wolf_run(), forest(), mountain(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wolf_run = on_battlefield(&engine, p0, kessig_wolf_run()).expect("Wolf Run deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert!(!keywords(&engine, elves).contains(KeywordSet::TRAMPLE));

    tap_mana_except(&mut engine, p0, wolf_run);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "a Forest, a Mountain and the Elves, which is {{X}}{{R}}{{G}} with X = 1"
    );
    activate(&mut engine, p0, kessig_wolf_run(), 1);

    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected the announced X, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 1));
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("announce X = 1");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, elves).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        pt(&engine, elves),
        (2, 1),
        "+X/+0 with X announced as 1, against the 1/1 it prints"
    );
    assert!(is_tapped(&engine, wolf_run));
}
