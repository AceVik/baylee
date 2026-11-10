//! `cards/creatures/mv_3/ley_druid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ley Druid — {2}{G}, a 1/1 Human Druid whose whole printed text is one line:
/// "{T}: Untap target land."
///
/// The board is built so three readings of that line disagree. Two tapped
/// Forests under the Druid's own seat say the untap is not "untap each land",
/// the Sol Ring beside them is a nonland permanent the word "land" must
/// decline, and the Forest across the table is the other half of "target land",
/// which names no controller at all. The mana floated before the activation is
/// read again afterwards, because the price is the tap symbol and no mana — an
/// ability that had spent it would read one or more lower.
#[test]
fn ley_druid_taps_to_untap_one_land_on_either_side_of_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ley_druid(), forest(), forest(), quiet_artifact()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let druid = on_battlefield(&engine, p0, ley_druid()).expect("the Druid is out");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 2, "two Forests under the Druid's seat");
    let (wanted, bystander) = (forests[0], forests[1]);

    // The Forests need a reason to stand back up, so every source is tapped
    // first — and the Druid is not one of them: `{{T}}: Untap target land` is
    // no mana ability, so the helper never presses it.
    tap_all_mana(&mut engine, p0);
    assert!(
        is_tapped(&engine, wanted) && is_tapped(&engine, bystander),
        "both Forests paid their {{T}}"
    );
    assert!(is_tapped(&engine, rock), "and so did the Sol Ring");
    assert!(
        !is_tapped(&engine, druid),
        "the Druid's own line is still paid for by a tap nobody has spent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Forests and the Sol Ring's own {{T}}: Add {{C}}{{C}}"
    );

    activate(&mut engine, p0, ley_druid(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated chooses");
    assert!(
        options.contains(&wanted) && options.contains(&bystander),
        "both of this seat's lands are on the menu: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target land\" names no controller, so a land across the table is as \
         legal as one of our own: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "the Sol Ring is an artifact and no land: {options:?}"
    );
    assert!(
        !options.contains(&druid),
        "and the Druid is a creature: a land is what this line turns back over: \
         {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "two Forests of ours and one of theirs, and nothing else on this board \
         is a land: {options:?}"
    );

    // CR 601.2c names the target and CR 601.2h pays afterwards, so the Druid
    // is still standing while the question is open.
    assert!(
        !is_tapped(&engine, druid),
        "the cost is the last step of the activation, not the first"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wanted],
            },
        )
        .expect("the Forest was one of the options it enumerated");

    assert!(is_tapped(&engine, druid), "{{T}} is what the line charges");
    assert!(
        is_tapped(&engine, wanted),
        "and untapping is no mana ability, so the named land is still down \
         while the ability sits on the stack"
    );
    assert!(!stack_is_empty(&engine), "which is where it went");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, wanted),
        "\"Untap target land\" — the land that was named stands back up"
    );
    assert!(
        is_tapped(&engine, bystander),
        "the Forest nobody named is still down, so this untaps one land and not \
         every land its controller has"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{T}} was the whole price: the mana the Forests and the Sol Ring \
         made is still floating, and an ability that had spent any of it would \
         read lower here"
    );
}
