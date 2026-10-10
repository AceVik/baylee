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
    // Only Wolf Run itself could still make mana, and its cost taps it, so
    // the pool is the bound.
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

/// X is announced with nothing floating, then paid in the activation's
/// payment window from the lands (CR 602.2b: CR 601.2b announces X, CR
/// 601.2g activates mana abilities while paying). It used to be bounded by
/// the floating pool, so with the mana still in the lands X was 0.
#[test]
fn kessig_wolf_run_announces_x_before_its_lands_are_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(115, forest())
        .battlefield(
            0,
            &[
                kessig_wolf_run(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wolf_run = on_battlefield(&engine, p0, kessig_wolf_run()).expect("Wolf Run deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, kessig_wolf_run(), 1);
    let Pending::ChooseNumber { max, .. } = engine.pending().clone() else {
        panic!("expected the announced X, got {:?}", engine.pending());
    };
    assert!(max >= 3, "five mana sources stand untapped, got max {max}");
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    assert!(
        engine.payment_window().is_some(),
        "the mana is made in the activation's window, got {:?}",
        engine.pending()
    );
    tap_mana_except(&mut engine, p0, wolf_run);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elves), (4, 1), "+3/+0 for X = 3");
    assert!(is_tapped(&engine, wolf_run));
}

/// Floating mana and untapped lands together pay one X: the Mountain's
/// {R} floats, the window makes the rest.
#[test]
fn kessig_wolf_run_pays_x_from_floating_mana_and_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(115, forest())
        .battlefield(
            0,
            &[
                kessig_wolf_run(),
                mountain(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wolf_run = on_battlefield(&engine, p0, kessig_wolf_run()).expect("Wolf Run deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    let red = on_battlefield(&engine, p0, mountain()).expect("Mountain deployed");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: red })
        .unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, kessig_wolf_run(), 1);
    let Pending::ChooseNumber { max, .. } = engine.pending().clone() else {
        panic!("expected the announced X, got {:?}", engine.pending());
    };
    assert!(max >= 2, "one floats and four sources stand, got max {max}");
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    tap_mana_except(&mut engine, p0, wolf_run);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "five made, {{2}}{{R}}{{G}} spent"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elves), (3, 1), "+2/+0 for X = 2");
}

/// A window left short reverses the activation and gives back the mana
/// made in it (CR 732.1): the lands untap, the pool is as it was, nothing
/// is on the stack, and the player keeps priority.
#[test]
fn kessig_wolf_run_left_short_gives_back_its_window() {
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

    activate(&mut engine, p0, kessig_wolf_run(), 1);
    engine.apply(p0, PlayerAction::ChooseNumber(10)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    assert!(engine.payment_window().is_some());
    tap_mana_except(&mut engine, p0, wolf_run);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    assert!(engine.payment_window().is_none());
    assert!(stack_is_empty(&engine), "nothing was activated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the mana is given back"
    );
    for &id in engine.state().zones.list(ZoneLocation::Battlefield) {
        assert!(!is_tapped(&engine, id), "every permanent untapped again");
    }
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the activator keeps priority, got {:?}",
        engine.pending()
    );
}
