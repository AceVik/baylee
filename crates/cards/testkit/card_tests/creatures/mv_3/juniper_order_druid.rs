//! `cards/creatures/mv_3/juniper_order_druid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Juniper Order Druid — {2}{G} 1/1 Human Cleric Druid: "{T}: Untap target
/// land." The whole card is that one line, and it carries two claims a board
/// can separate: the tap is the price, and the target is *any* land. So the
/// Druid is cast off four Forests — which is exactly what leaves lands tapped
/// for it to untap — and a turn is walked, because a creature's {T} is not
/// payable before its controller's next turn begins (CR 302.6). The
/// opponent's own lands are tapped on their own turn, so the land this test
/// names lies across the table: the ability stands it back up, while every
/// land it did not name is still lying tapped — the single-target half of the
/// same sentence.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn juniper_order_druid_taps_itself_to_untap_one_land_of_either_seat() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[juniper_order_druid()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches their own main phase"
    );

    // {2}{G} off the four Forests. Tapping them is the point of the board: a
    // land already tapped is the only thing "untap target land" can be seen
    // to do.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, four green — the Druid prints no mana of its own"
    );
    cast_with_floating(&mut engine, p0, juniper_order_druid());
    pass_until(&mut engine, stack_is_empty);
    let druid = on_battlefield(&engine, p0, juniper_order_druid()).expect("the Druid resolved");
    assert_eq!(pt(&engine, druid), (1, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{G}} came out of the four green"
    );

    // The Druid arrived this turn, so its {{T}} is not payable yet (CR 302.6).
    // The opponent's main phase is also where their lands can be tapped by
    // hand, which is what puts a tapped land on the far side of the table.
    assert!(
        walk_to_own_main(&mut engine, p1),
        "p1 reaches their own main phase"
    );
    let theirs = on_battlefield(&engine, p1, forest()).expect("an opponent's Forest is out");
    tap_all_mana(&mut engine, p1);
    assert!(
        is_tapped(&engine, theirs),
        "the opponent tapped their own lands for mana"
    );
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Druid's controller takes another turn"
    );
    assert!(!is_tapped(&engine, druid), "and the Druid untapped with it");

    let mine = all_on_battlefield(&engine, p0, forest());
    assert_eq!(mine.len(), 4, "the four Forests that paid for the Druid");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four tapped Forests and nothing else on this board makes mana"
    );
    assert!(
        !is_tapped(&engine, druid),
        "the helper left it standing, because the Druid is no mana source: \
         its {{T}} is the price the ability below is about to charge"
    );
    assert!(
        is_tapped(&engine, theirs),
        "and the Forest across the table is still tapped"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the Druid's controller holds priority");
    assert!(
        legal.abilities.contains(&(druid, 0)),
        "`{{T}}: Untap target land` — the Druid's own tap is the whole price, \
         so the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, juniper_order_druid(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"untap target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert_eq!((min, max), (1, 1), "exactly one land");
    assert!(
        options.contains(&theirs),
        "the sentence says \"target land\" and not \"a land you control\": the \
         opponent's tapped Forest is on the menu: {options:?}"
    );
    assert!(
        mine.iter().all(|id| options.contains(id)),
        "and so is every Forest of the activating seat: {options:?}"
    );
    assert!(
        !options.contains(&druid),
        "the Druid is a creature and no land: {options:?}"
    );
    assert_eq!(
        options.len(),
        6,
        "the six lands on the table and nothing else: {options:?}"
    );
    assert!(
        !is_tapped(&engine, druid),
        "CR 601.2c before CR 601.2h: the tap is still unpaid while the target \
         question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Forest across the table was one of the options it enumerated");
    assert!(
        is_tapped(&engine, druid),
        "{{T}} is the last step of the activation, paid by the Druid itself"
    );
    assert!(
        !stack_is_empty(&engine),
        "untapping a permanent is no mana ability, so the ability uses the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, theirs),
        "\"untap target land\": the Forest that lay across the table stood back up"
    );
    assert!(
        mine.iter().all(|id| is_tapped(&engine, *id)),
        "and only the land that was named: every Forest the ability did not \
         target is still lying tapped"
    );
    assert!(
        on_battlefield(&engine, p0, juniper_order_druid()).is_some(),
        "an activated ability costs the Druid nothing but the tap it already paid"
    );
}
