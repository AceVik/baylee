//! `cards/creatures/mv_4/goblin_clearcutter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Goblin Clearcutter` is a 3/3 creature under `Coverage::Implemented` with a mana ability costing `{T}` and sacrificing a Forest.
/// When activated, it prompts with `ChoicePrompt::CostSacrifice` offering only controlled Forests, excluding non-Forest lands and opponent lands.
/// After the sacrifice, three consecutive color choices between Red and Green add three mana in the chosen combination without using the stack.
#[test]
fn goblin_clearcutter_sacrifices_a_forest_for_three_mana_in_any_combination_of_red_and_green() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_clearcutter(), forest(), mountain()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let clearcutter =
        on_battlefield(&engine, p0, goblin_clearcutter()).expect("Clearcutter deployed");
    let my_forest = on_battlefield(&engine, p0, forest()).expect("my Forest deployed");
    let my_mountain = on_battlefield(&engine, p0, mountain()).expect("my Mountain deployed");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest deployed");

    assert!(
        !is_tapped(&engine, clearcutter),
        "Clearcutter starts untapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "mana pool is empty initially"
    );

    activate(&mut engine, p0, goblin_clearcutter(), 0);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected CostSacrifice prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "activating player chooses land to sacrifice");
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert_eq!((min, max), (1, 1), "sacrifice exactly one Forest");
    assert!(
        options.contains(&my_forest),
        "controlled Forest is an option: {options:?}"
    );
    assert!(
        !options.contains(&my_mountain),
        "Mountain is not a Forest: {options:?}"
    );
    assert!(
        !options.contains(&their_forest),
        "opponent's Forest is not controlled by activator: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_forest],
            },
        )
        .expect("sacrificing Forest is legal");

    // Three picks of Red and/or Green for the three mana combination.
    for i in 0..3 {
        let Pending::ChooseColor {
            player: color_player,
            options,
        } = engine.pending().clone()
        else {
            panic!(
                "expected ChooseColor for pick {i}, got {:?}",
                engine.pending()
            );
        };
        assert_eq!(color_player, p0);
        assert_eq!(
            options,
            vec![ManaColor::Red, ManaColor::Green],
            "offers Red and Green"
        );
        let choice = if i == 0 {
            ManaColor::Red
        } else {
            ManaColor::Green
        };
        engine
            .apply(p0, PlayerAction::ChooseColor(choice))
            .expect("color choice is legal");
    }

    assert!(
        stack_is_empty(&engine),
        "mana abilities do not use the stack"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "one red mana chosen");
    assert_eq!(pool.available(ManaColor::Green), 2, "two green mana chosen");
    assert_eq!(pool.total(), 3, "total of three mana added");

    assert!(
        is_tapped(&engine, clearcutter),
        "Goblin Clearcutter tapped to activate"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "sacrificed Forest is in the graveyard"
    );
}
