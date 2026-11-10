//! `cards/creatures/mv_2/kami_of_ancient_law.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kami of Ancient Law is a 2/2 whose whole printed text is one line:
/// "Sacrifice this creature: Destroy target enchantment." Both halves want
/// their own board. The price is the source itself, so the Kami is still
/// standing while the target question is open (CR 601.2c) and in its owner's
/// graveyard the instant that question is answered (CR 601.2h) — a rider that
/// resolved later would look exactly the same once the stack had emptied.
/// And the menu is what reads `Filter::ENCHANTMENT`: the opponent's
/// enchantment is the only legal target, while the creature on each side of
/// the table is a permanent that any "target permanent" reading would have
/// offered and this one must not.
#[test]
fn kami_of_ancient_law_eats_itself_to_destroy_an_enchantment_across_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kami_of_ancient_law(), llanowar_elves()])
        .battlefield(1, &[their_enchantment(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let kami = on_battlefield(&engine, p0, kami_of_ancient_law()).expect("the Kami is out");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let enchantment =
        on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");
    assert_eq!(pt(&engine, kami), (2, 2), "the printed 2/2 body");

    // The whole price is the source itself — no mana and no tap — so the line
    // is offered off a board with an empty pool and nothing floating for it.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that holds the Kami holds priority");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cost is a sacrifice and no mana, so nothing has to be paid into a pool first"
    );
    assert!(
        legal.abilities.contains(&(kami, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, kami_of_ancient_law(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert_eq!(
        options,
        vec![enchantment],
        "the one enchantment on the table and nothing else: the Kami and both \
         Elves are creatures, which is exactly what a bare `Filter::Any` would \
         have put on this menu"
    );
    assert!(
        !options.contains(&my_elf) && !options.contains(&their_elf),
        "a creature is not an enchantment, on either side of the table: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, kami_of_ancient_law()).is_some(),
        "CR 601.2c before CR 601.2h: the target is named while the Kami is \
         still standing, so the sacrifice has not been paid yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![enchantment],
            },
        )
        .expect("the enchantment the question offered was one of its answers");

    assert!(
        in_graveyard(&engine, p0, kami_of_ancient_law()).is_some(),
        "the sacrifice is the last step of the activation, so it is paid the \
         moment the target question is settled — and a sacrificed creature \
         goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the ability that price paid for is what is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the targeted enchantment was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and it rests in the graveyard of the seat that owned it, not the \
         seat that aimed the ability"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and neither did the creature on this side of the table"
    );
}
