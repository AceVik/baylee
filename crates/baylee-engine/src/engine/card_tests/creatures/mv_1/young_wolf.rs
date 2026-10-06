//! `cards/creatures/mv_1/young_wolf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Young Wolf is a printed 1/1 Wolf for {G}, and its only rules text is
/// undying. The scenario is unchanged and the ending is the opposite one:
/// this test read the `Coverage::Partial` gap — "and simply stays in the
/// graveyard" — until undying existed, and the day it did is the day the
/// assertion had to invert.
///
/// What it still buys over `engine::undying_tests` is the *door*. A cost
/// that sacrifices is dying (CR 700.4) and Ashnod's Altar eats the Wolf as
/// a cost rather than as an effect, so the trigger is watching the zone
/// change and not the destruction that usually causes it. An exile would be
/// no proof — a card exiled never triggers undying either.
#[test]
fn young_wolf_is_sacrificed_to_the_altar_and_undying_returns_it_bigger() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ashnods_altar(), forest()])
        .hand(0, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {G} off the Forest. The printed 1/1 body is the whole of what the
    // engine implements of this card.
    cast_from_hand(&mut engine, p0, young_wolf());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("the Wolf resolved");
    assert_eq!(pt(&engine, wolf), (1, 1), "the printed 1/1 body");
    assert!(
        types(&engine, wolf).contains(TypeSet::CREATURE),
        "and it is a creature"
    );

    // The Altar's cost is "sacrifice a creature," which CR 700.4 counts as
    // dying — the very event undying watches for.
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "the sacrifice asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&wolf),
        "the Wolf is the creature the Altar may eat: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wolf],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let back = on_battlefield(&engine, p0, young_wolf())
        .expect("a sacrifice is a death, so undying returned it");
    assert_eq!(
        pt(&engine, back),
        (2, 2),
        "the printed 1/1 with the +1/+1 counter it returns with"
    );
    assert!(
        in_graveyard(&engine, p0, young_wolf()).is_none(),
        "and it is on the table rather than in the graveyard as well"
    );
}
