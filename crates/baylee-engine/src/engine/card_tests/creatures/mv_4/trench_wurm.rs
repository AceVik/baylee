//! `cards/creatures/mv_4/trench_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Trench Wurm — {3}{B} — a 3/3 Wurm printing "{2}{R}, {T}: Destroy target
/// nonbasic land."
///
/// Both prices are played rather than read: four Swamps pay the {3}{B} for
/// real, and the Wurm is then walked to a turn of its own, because a creature
/// with summoning sickness cannot pay a {T} (CR 302.6) — only a Wurm that has
/// been under its controller's control since the turn began can show that the
/// tap symbol is part of the card at all. The target menu is the sentence
/// itself: the Badlands across the table is a nonbasic land and is the whole
/// of the menu, while the Forest beside it is a basic land and is not on there
/// — the half a filter that had lost the word would keep passing unnoticed —
/// and the Wurm is a creature and no land. The {2}{R} is claimed off a pool
/// that is already floating (CR 601.2h), the {T} is read only once the target
/// has been answered (CR 601.2c before CR 601.2h), and the destroyed land goes
/// to the graveyard of the seat that *owns* it and not the one that aimed.
#[test]
#[allow(clippy::too_many_lines)]
fn trench_wurm_taps_and_two_red_to_destroy_a_nonbasic_land_and_not_a_basic_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), mountain()])
        .hand(0, &[trench_wurm()])
        // One nonbasic land and one basic land across the table: "nonbasic" is
        // the word that separates them, and a pair on the same board is the
        // only thing that can read it.
        .battlefield(1, &[badlands(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let spell_land = on_battlefield(&engine, p1, badlands()).expect("their Badlands is out");
    let basic_land = on_battlefield(&engine, p1, forest()).expect("a basic Forest is beside it");

    // {3}{B} off the four Swamps, with the Mountain named as the printing kept
    // back: it is the red the activated line needs, and nothing has to be
    // spent twice because it untaps like everything else on the turn below.
    tap_all_mana_but(&mut engine, p0, Some(mountain()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps tapped, and the Mountain left standing"
    );
    cast_with_floating(&mut engine, p0, trench_wurm());
    pass_until(&mut engine, stack_is_empty);
    let wurm = on_battlefield(&engine, p0, trench_wurm()).expect("the Wurm resolved");
    assert_eq!(pt(&engine, wurm), (3, 3), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}}{{B}} took all four of the black the Swamps made"
    );

    // A creature that entered this turn cannot pay a {T} (CR 302.6), so the
    // turn the Wurm's controller actually gets is what makes the ability
    // reachable at all.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 takes another turn with the Wurm on the table"
    );
    assert!(
        !is_tapped(&engine, wurm),
        "the untap step stood the Wurm back up"
    );

    // `{2}{R}` is read off the *pool*: `legal.abilities` is filtered through
    // `can_afford`, which looks at the pool and not at the untapped lands.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 5, "four Swamps and one Mountain");
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the one red source on the board"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(wurm, 0)),
        "the one line the card prints is offered now that its {{2}}{{R}} is \
         floating: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, trench_wurm(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target nonbasic land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one land, and the ability asks once");
    assert!(
        options.contains(&spell_land),
        "the Badlands is a nonbasic land and is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&basic_land),
        "\"nonbasic\" is read, not skipped: the Forest beside it is a basic \
         land and no legal target: {options:?}"
    );
    assert!(
        !options.contains(&wurm),
        "the Wurm is a creature and no land: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and that Badlands is the whole menu — every other permanent on this \
         board is a basic land or the Wurm itself: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, wurm),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "and the {{2}}{{R}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell_land],
            },
        )
        .expect("the land the question offered was chosen");

    assert!(
        is_tapped(&engine, wurm),
        "{{T}} is paid by the Wurm itself — the half of the price a creature \
         with summoning sickness had to wait a turn for"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{2}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying a land is no mana ability, so the ability is on the stack"
    );
    assert!(
        on_battlefield(&engine, p1, badlands()).is_some(),
        "and nothing has happened to the land yet: the destruction is the \
         resolution, not the cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, badlands()).is_some(),
        "the targeted land was destroyed, and it goes to the graveyard of the \
         seat that owns it"
    );
    assert!(
        in_graveyard(&engine, p0, badlands()).is_none(),
        "not to the graveyard of the seat that aimed the ability"
    );
    assert!(
        on_battlefield(&engine, p1, badlands()).is_none(),
        "and it has left the battlefield, which is what destruction means"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the basic land the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, swamp()).is_some(),
        "nor did any land of the seat that paid for it"
    );
    assert!(
        on_battlefield(&engine, p0, trench_wurm()).is_some(),
        "an activated ability costs the Wurm nothing but its tap"
    );
}
