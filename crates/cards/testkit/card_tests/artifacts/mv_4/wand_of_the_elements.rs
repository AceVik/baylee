//! `cards/artifacts/mv_4/wand_of_the_elements.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Wand of the Elements` is an artifact under `Coverage::Implemented` with two activated abilities costing `{T}` and sacrificing a typed land.
/// Activating ability 0 prompts with `ChoicePrompt::CostSacrifice` to sacrifice an Island controlled by the activator.
/// Non-Island lands and opponent-controlled lands are excluded from the sacrifice menu.
/// Upon resolution, a 2/2 blue Elemental creature token with flying is created.
#[test]
fn wand_of_the_elements_sacrifices_an_island_to_create_flying_elemental() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wand_of_the_elements(), island(), forest()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wand = on_battlefield(&engine, p0, wand_of_the_elements()).expect("Wand deployed");
    let my_island = on_battlefield(&engine, p0, island()).expect("my Island deployed");
    let my_forest = on_battlefield(&engine, p0, forest()).expect("my Forest deployed");
    let their_island = on_battlefield(&engine, p1, island()).expect("their Island deployed");

    assert!(!is_tapped(&engine, wand), "Wand enters untapped");
    assert_eq!(tokens_of(&engine, p0).len(), 0, "no tokens initially");

    // Ability 0 costs {T}, Sacrifice an Island: no mana is needed.
    activate(&mut engine, p0, wand_of_the_elements(), 0);

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
    assert_eq!(
        player, p0,
        "activating player chooses permanent to sacrifice"
    );
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert_eq!((min, max), (1, 1), "sacrifice exactly one land");
    assert!(
        options.contains(&my_island),
        "controlled Island is an option: {options:?}"
    );
    assert!(
        !options.contains(&my_forest),
        "controlled Forest is not an Island: {options:?}"
    );
    assert!(
        !options.contains(&their_island),
        "opponent's Island is not controlled by activator: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_island],
            },
        )
        .expect("sacrificing controlled Island is legal");

    assert!(is_tapped(&engine, wand), "Wand tapped as activation cost");
    assert!(
        on_battlefield(&engine, p0, island()).is_none(),
        "sacrificed Island left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, island()).is_some(),
        "sacrificed Island was moved to graveyard"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one token was created");
    let elemental = tokens[0];
    assert_eq!(pt(&engine, elemental), (2, 2), "Elemental token is 2/2");
    assert!(
        keywords_of(&engine, elemental).contains(KeywordSet::FLYING),
        "Elemental token has flying"
    );
}
