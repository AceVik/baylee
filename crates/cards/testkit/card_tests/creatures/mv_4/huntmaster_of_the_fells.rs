//! `cards/creatures/mv_4/huntmaster_of_the_fells.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Huntmaster of the Fells: a Wolf and 2 life as it enters; at an upkeep
/// after a turn with no spells it becomes Ravager of the Fells (4/4 trample);
/// after a turn in which a player cast two it turns back, and transforming
/// into Huntmaster makes another Wolf and 2 life. One spell in a turn does
/// neither.
#[test]
fn huntmaster_of_the_fells_flips_on_quiet_turns_and_back_on_busy_ones() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[mountain(), forest(), mountain(), forest(), swamp(), swamp()],
        )
        .hand(
            0,
            &[huntmaster_of_the_fells(), dark_ritual(), dark_ritual()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_mana_where(&mut engine, p0, |_| true);
    cast_with_floating(&mut engine, p0, huntmaster_of_the_fells());
    pass_until(&mut engine, stack_is_empty);
    let hunt = on_battlefield(&engine, p0, huntmaster_of_the_fells()).expect("resolved");
    assert_eq!(tokens_of(&engine, p0).len(), 1, "a Wolf on entering");
    assert_eq!(pt(&engine, tokens_of(&engine, p0)[0]), (2, 2));
    assert_eq!(engine.state().players[0].life, 22);

    // Turn 1 had a spell in it: the opponent's upkeep leaves it alone.
    through_upkeep_of(&mut engine, p1);
    assert_eq!(face_shown(&engine, hunt), 0, "one spell was cast last turn");

    // Turn 2 had none: at our upkeep it turns over.
    through_upkeep_of(&mut engine, p0);
    assert_eq!(face_shown(&engine, hunt), 1, "Ravager of the Fells");
    assert_eq!(pt(&engine, hunt), (4, 4));
    assert!(keywords(&engine, hunt).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "no Wolf for becoming Ravager"
    );

    // Two spells this turn: at the opponent's upkeep it turns back, and
    // "transforms into Huntmaster of the Fells" makes a Wolf and 2 life.
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    through_upkeep_of(&mut engine, p1);
    assert_eq!(face_shown(&engine, hunt), 0, "a player cast two spells");
    assert_eq!(tokens_of(&engine, p0).len(), 2, "a second Wolf");
    assert_eq!(engine.state().players[0].life, 24, "and 2 more life");
}

/// No last turn at the first upkeep of the game: a Huntmaster that starts
/// on the battlefield stays as it is there, and flips at the next upkeep
/// after a turn nobody cast anything in.
#[test]
fn huntmaster_of_the_fells_does_not_flip_at_the_first_upkeep_of_the_game() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[huntmaster_of_the_fells()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    let hunt = on_battlefield(&engine, PlayerId::new(0), huntmaster_of_the_fells()).unwrap();
    assert_eq!(face_shown(&engine, hunt), 0);
    through_upkeep_of(&mut engine, p1);
    assert_eq!(face_shown(&engine, hunt), 1, "nothing was cast in turn 1");
    assert!(tokens_of(&engine, PlayerId::new(0)).is_empty());
}

/// "Whenever this creature transforms into Ravager of the Fells, it deals 2
/// damage to target opponent or planeswalker and 2 damage to up to one target
/// creature that player or that planeswalker's controller controls." The
/// first question offers the opponent and never its controller; once the
/// opponent is named, the second offers that opponent's creatures and none
/// of Ravager's controller's. The opponent takes 2 and so does the creature
/// picked (a 2/2 Steadfast Guard, which dies).
#[test]
fn ravager_of_the_fells_burns_an_opponent_and_one_of_that_players_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[huntmaster_of_the_fells(), llanowar_elves()])
        .battlefield(1, &[steadfast_guard(), thundering_giant()])
        .start();
    keep_mulligans(&mut engine);
    let hunt = on_battlefield(&engine, p0, huntmaster_of_the_fells()).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let guard = on_battlefield(&engine, p1, steadfast_guard()).unwrap();
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();

    // Nothing was cast in turn 1: at the opponent's upkeep it turns over.
    let (objects, players) = upkeep_until_ravager_asks(&mut engine, p1);
    assert_eq!(face_shown(&engine, hunt), 1, "Ravager of the Fells");
    assert_eq!(players, vec![p1], "target opponent: not its controller");
    assert!(objects.is_empty(), "no planeswalker on the board");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent");

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("the second target is asked: {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 1), "up to one target creature");
    assert!(
        options.contains(&guard) && options.contains(&giant),
        "that player's creatures"
    );
    assert!(
        !options.contains(&elves),
        "not a creature another player controls"
    );
    assert!(!options.contains(&hunt));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![guard],
            },
        )
        .expect("one of theirs");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "2 damage to the opponent"
    );
    assert!(
        on_battlefield(&engine, p1, steadfast_guard()).is_none(),
        "2 damage to the 2/2"
    );
    assert!(on_battlefield(&engine, p1, thundering_giant()).is_some());
}

/// The second target is "up to one": declined, the opponent still takes 2
/// and no creature is dealt anything.
#[test]
fn ravager_of_the_fells_may_leave_the_creature_out() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[huntmaster_of_the_fells()])
        .battlefield(1, &[steadfast_guard()])
        .start();
    keep_mulligans(&mut engine);
    let _ = upkeep_until_ravager_asks(&mut engine, p1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("none is an answer to up to one");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 18);
    let guard = on_battlefield(&engine, p1, steadfast_guard()).expect("untouched");
    assert_eq!(engine.state().object(guard).unwrap().damage, 0);
}
