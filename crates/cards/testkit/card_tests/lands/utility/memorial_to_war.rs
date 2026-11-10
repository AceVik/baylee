//! `cards/lands/utility/memorial_to_war.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Memorial to War prints three lines: it enters tapped, it taps for `{R}`, and
/// `{4}{R}, {T}, Sacrifice this land: Destroy target land`.
///
/// The entry has to be played and not seated: `starting_battlefield` moves a
/// permanent with `Cause::Setup`, which is a placement rather than an entry, so
/// no replacement effect ever looks at it and a board built that way would show
/// an untapped Memorial whatever the card says. The other two lines are read on
/// the turns after a real `PlayLand`, and the destroy is only claimed once the
/// mana is floating, because `legal.abilities` is filtered through
/// `can_afford` and that reads the pool rather than the untapped lands.
#[test]
#[allow(clippy::too_many_lines)]
fn memorial_to_war_enters_tapped_taps_for_red_and_trades_itself_for_a_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[memorial_to_war()])
        // A land across the table, which is what "target land" has to reach.
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real `PlayLand` out of the hand, so the printed entry modifier runs:
    // the same permanent seated with `Cause::Setup` would arrive untapped.
    let land = play_land(&mut engine, p0, memorial_to_war());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");

    // Every line the card prints is paid for with the tap symbol, so a land
    // that has just entered tapped is offered neither of them.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the seat holds priority again after its land drop, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "an untapped land is a paid {{T}}, and this one is tapped: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "and {{4}}{{R}} is not in a pool that an empty board never filled: {:?}",
        legal.abilities
    );

    // One turn round the table: the untap step is what stands the land up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the Memorial back up"
    );

    // Ability 0: "{{T}}: Add {{R}}". A fixed colour, so nothing is asked on the
    // way and the mana is in the pool the moment the tap resolves (CR 605.3b).
    activate(&mut engine, p0, memorial_to_war(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "{{T}}: Add {{R}}");
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap: the five Mountains beside it are still standing"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting"
    );
    assert!(is_tapped(&engine, land), "the Memorial paid its own {{T}}");

    // Across the table and back: the Memorial untaps again, and the red the
    // last turn left floating went with the step that ended (CR 500.5).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "a second untap step, and the land is standing and ready"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool is empty, so what the ability below charges has to be made"
    );

    let their_forest = on_battlefield(&engine, p1, forest()).expect("their land is out");
    let my_mountain = on_battlefield(&engine, p0, mountain()).expect("my Mountains are out");

    // `legal.abilities` is filtered through `can_afford`, and that reads the
    // pool rather than the untapped lands: five untapped Mountains and an
    // untapped Memorial still offer nothing while nothing is floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "{{4}}{{R}} is not in the pool, so the cost is unpayable and the line \
         is withheld: {:?}",
        legal.abilities
    );

    // The Memorial is named as the printing kept back: it must still be
    // untapped to pay its own {{T}} for the destroy.
    tap_all_mana_but(&mut engine, p0, Some(memorial_to_war()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains tapped, and the Memorial kept back for its own {{T}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "with five red floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, memorial_to_war(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&their_forest) && options.contains(&my_mountain),
        "\"target land\" is any land, on either side of the table: {options:?}"
    );
    // CR 601.2c before CR 601.2h: the target is named while the Memorial is
    // still standing on the battlefield and the five red are still floating.
    assert!(
        on_battlefield(&engine, p0, memorial_to_war()).is_some(),
        "the price is paid after the target, not before it"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_forest],
            },
        )
        .expect("the land was one of the options the question enumerated");

    assert!(
        in_graveyard(&engine, p0, memorial_to_war()).is_some(),
        "\"Sacrifice this land\" is part of the price, so the card is in its \
         owner's graveyard rather than merely gone"
    );
    assert!(
        on_battlefield(&engine, p0, memorial_to_war()).is_none(),
        "and it has left the battlefield, which is the sacrifice itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying a land is no mana ability, so the ability waits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "the land the ability named was destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "and it is no longer on the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, mountain()).is_some(),
        "one target, one land: the Mountains the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, memorial_to_war()).is_none(),
        "the Memorial is gone — the price was the card itself, not just its tap"
    );
}
