//! `cards/creatures/mv_4/cackling_imp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cackling Imp — {2}{B}{B} 2/2 Imp with flying and "{T}: Target player loses
/// 1 life." The whole price is the tap symbol and no mana at all, which is why
/// the pool is read empty on both sides of the activation: the line is offered
/// with nothing floating, and it still resolves off the stack, because losing
/// life is no mana ability (CR 605.1b) and the {T} is paid by the permanent
/// rather than by a cost the pool could have covered. "Target player" is every
/// seat at the table and not "target opponent", so both are asserted to be on
/// the menu before the seat across it is named — and the seat that aimed the
/// ability keeps its own life, which is the control that says the target was
/// the choice it looks like.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn cackling_imp_taps_to_take_a_life_from_the_player_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cackling_imp()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let imp = on_battlefield(&engine, p0, cackling_imp()).expect("the Imp is on the table");
    assert_eq!(pt(&engine, imp), (2, 2), "the printed 2/2 body");
    assert!(
        keywords(&engine, imp).contains(KeywordSet::FLYING),
        "and the printed flying reaches the permanent"
    );
    assert!(!is_tapped(&engine, imp), "nothing has tapped it yet");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the only price the card prints is its own tap symbol"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(imp, 0)),
        "{{T}} and no mana is a payable price, so the one line the card prints \
         is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, cackling_imp(), 0);
    if let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    {
        assert_eq!(player, p0, "the activating seat names the target");
        assert!(
            options.is_empty(),
            "\"target player\" offers players and no object at all: {options:?}"
        );
        assert!(
            player_options.contains(&p0) && player_options.contains(&p1),
            "\"target player\" is every seat at the table, the activating one \
             included: {player_options:?}"
        );
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .expect("the seat across the table was one of the options");
    } else if let Pending::ChoosePlayer { player, options } = engine.pending().clone() {
        assert_eq!(player, p0, "the activating seat names the target");
        assert!(
            options.contains(&p0) && options.contains(&p1),
            "\"target player\" is every seat at the table: {options:?}"
        );
        engine
            .apply(player, PlayerAction::ChoosePlayer(p1))
            .expect("the seat across the table was one of the options");
    } else {
        panic!(
            "the ability asks which player it drains, got {:?}",
            engine.pending()
        );
    }

    // CR 601.2h: the {T} is the last step of the activation, so both halves of
    // the price are read after the target question has been answered.
    assert!(is_tapped(&engine, imp), "{{T}} was the whole of the price");
    assert!(
        !stack_is_empty(&engine),
        "losing life is no mana ability, so the ability is on the stack"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and nothing has resolved yet"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"target player loses 1 life\" — one, off the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "while the seat that aimed it keeps its own life: \"target player\" was \
         read as the choice it prints, not as \"you\""
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the tap paid the whole price, so no mana was spent either"
    );
    assert!(
        on_battlefield(&engine, p0, cackling_imp()).is_some(),
        "an activated ability costs the creature nothing but its tap"
    );
}
