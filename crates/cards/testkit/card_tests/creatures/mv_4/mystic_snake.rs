//! `cards/creatures/mv_4/mystic_snake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mystic Snake — {1}{G}{U}{U}, a 2/2 Snake with flash and one triggered
/// sentence: "When this creature enters, counter target spell."
///
/// Flash is the half that makes the other half reachable: the Snake is cast
/// while a spell is already on the stack on an opponent's turn, and its
/// enters-trigger then aims at the very spell it interrupted. Countering is
/// read where a countered spell leaves a mark — the Dark Ritual in its owner's
/// graveyard, the Swamp that paid for it still tapped, and a mana pool that
/// never receives the three black mana an uncountered Ritual prints.
#[test]
fn mystic_snake_flashes_in_to_counter_the_spell_it_answered() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), island(), island(), island()])
        .hand(0, &[mystic_snake()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // p1 is handed priority in p0's turn and spends its one Swamp on the
    // Ritual: {B} paid, three black mana printed for the resolution.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    cast_from_hand(&mut engine, p1, dark_ritual());
    let ritual = on_stack(&engine, dark_ritual()).expect("the Ritual is on the stack");

    // ... and p0 answers it with a creature, which is the whole of flash.
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let snake = in_hand(&engine, p0, mystic_snake()).expect("the Snake is in hand");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "{{1}}{{G}}{{U}}{{U}} out of four lands, and nothing else on the board"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&snake),
        "a creature with flash is castable with a spell already on the stack \
         on an opponent's turn, where a sorcery-speed creature could not be: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, mystic_snake());

    // The Snake resolves and its enters-trigger asks what it is aimed at: the
    // only spell on the stack is the one it was flashed in to answer.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the Snake's controller aims its own trigger");
    assert_eq!(
        options,
        vec![ritual],
        "\"counter target spell\" finds the spell still waiting below the trigger"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .expect("the spell the trigger offered is the one it counters");

    pass_until(&mut engine, stack_is_empty);

    let landed = on_battlefield(&engine, p0, mystic_snake()).expect("the Snake resolved");
    assert_eq!(pt(&engine, landed), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, landed).contains(KeywordSet::FLASH),
        "flash is a keyword of the permanent it becomes, not only of the cast"
    );
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "a countered spell is put into its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "and the three black mana it prints were never added: the spell never resolved"
    );
    assert!(
        is_tapped(
            &engine,
            on_battlefield(&engine, p1, swamp()).expect("the Swamp is still out")
        ),
        "the {{B}} cost was still paid — the counter stops the effect, not the price"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{G}}{{U}}{{U}} the Snake cost came out of the pool"
    );
}
