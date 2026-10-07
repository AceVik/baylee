//! `cards/creatures/mv_1/elvish_lyrist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elvish Lyrist — {G}, a 1/1 Elf — prints one line: "{G}, {T}, Sacrifice
/// this creature: Destroy target enchantment." Each part of that price is
/// visible from a different angle, which is what one scenario has to read at
/// once: the {G} is a real payment out of a pool one Forest filled, the {T}
/// and the sacrifice both leave the Elf in its owner's graveyard, and
/// "target enchantment" is what decides the menu — the Enchantment across
/// the table is on it and the Trample Wurm beside it, a permanent with no
/// such type, is not. Reading the card says the Elf dies to its own ability;
/// playing it says the same thing, and also that the Wurm lives.
#[test]
fn elvish_lyrist_spends_its_mana_its_tap_and_itself_to_destroy_an_enchantment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), elvish_lyrist(), rootbreaker_wurm()])
        .battlefield(1, &[fastbond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is out");
    let bond =
        on_battlefield(&engine, p1, fastbond()).expect("the Enchantment is across the table");

    // The Forest is the whole pool. The Lyrist prints no mana ability and the
    // Wurm none either, so "one green" is exactly what the one Forest made —
    // and the {G} the ability charges is paid out of what is floating.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one Forest, one green mana"
    );

    // Ability 0 is the card's only line; `activate` panics unless the offer
    // carries it, which is itself the affordability half of the price.
    activate(&mut engine, p0, elvish_lyrist(), 0);

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the destruction targets an enchantment, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses the target");
    assert!(
        options.contains(&bond),
        "the Enchantment across the table is what the ability is for: {options:?}"
    );
    assert!(
        !options.contains(&wurm),
        "a creature is no enchantment: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and that one is the whole menu: {options:?}"
    );

    // Target chosen (CR 601.2c); the costs come last (CR 601.2h), so while
    // the question stands the mana is still floating and the Elf is still on
    // the battlefield.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "no cost is paid while the target question stands"
    );
    assert!(
        on_battlefield(&engine, p0, elvish_lyrist()).is_some(),
        "and the Elf has not sacrificed itself yet"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bond],
            },
        )
        .expect("the Enchantment was one of the options");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} was spent paying the ability"
    );
    assert!(
        on_battlefield(&engine, p0, elvish_lyrist()).is_none(),
        "the sacrifice is part of the price, so the Lyrist left the table"
    );
    assert!(
        in_graveyard(&engine, p0, elvish_lyrist()).is_some(),
        "and a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, fastbond()).is_none(),
        "\"destroy target enchantment\" took the Enchantment off the table"
    );
    assert!(
        in_graveyard(&engine, p1, fastbond()).is_some(),
        "into its owner's graveyard, not the Lyrist's"
    );
    assert!(
        on_battlefield(&engine, p0, rootbreaker_wurm()).is_some(),
        "the ability reached the permanent it named and no other"
    );
}
