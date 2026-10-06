//! `cards/enchantments/mv_3/aura_fracture.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aura Fracture — {2}{W} enchantment: "Sacrifice a land: Destroy target
/// enchantment."
///
/// Both halves of that line are the engine's answer rather than the card's,
/// so the board is built to strike each one. The sacrifice names no land in
/// particular: the menu is the four Plains under this seat and nothing else —
/// not the Forest across the table (CR 701.21a), and not the Aura Fracture
/// itself, which is an enchantment and no land. The target menu holds the
/// enchantment across the table *and* the Fracture on this side, because
/// "target enchantment" reaches both, while every land on either board stays
/// off it. And the order is CR 601.2c before CR 601.2h, so while the target
/// question stands nothing has been paid and nothing has been destroyed —
/// the land is still untapped and the enchantment is still standing, and only
/// one of them leaves the battlefield.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn aura_fracture_trades_a_land_for_the_enchantment_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[aura_fracture()])
        // An enchantment across the table to destroy, and a Forest that
        // "sacrifice a land" has to decline because it is not this seat's.
        .battlefield(1, &[fastbond(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{W} off three Plains, with the fourth kept back: the land the price
    // is paid with is untapped, so the sacrifice is read off a land nobody
    // could mistake for a tapped-out one.
    let fodder = on_battlefield(&engine, p0, plains()).expect("a Plains of mine is out");
    tap_mana_except(&mut engine, p0, fodder);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains tapped, and the fourth held back"
    );
    cast_with_floating(&mut engine, p0, aura_fracture());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, aura_fracture()).is_some()
    });
    let fracture = on_battlefield(&engine, p0, aura_fracture()).expect("Aura Fracture resolved");
    let doom = on_battlefield(&engine, p1, fastbond()).expect("the enchantment across the table");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let my_lands = all_on_battlefield(&engine, p0, plains());
    assert_eq!(my_lands.len(), 4, "four Plains, all of them still standing");
    assert!(
        !is_tapped(&engine, fodder),
        "the Plains kept back never paid for the enchantment"
    );

    // The whole price is a land and no mana, so the offer turns on nothing
    // the pool could have supplied.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(fracture, 0)),
        "a land to give up is the whole price, so the one line the card \
         prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, aura_fracture(), 0);

    // CR 601.2c: the target is named first, and nothing is paid while the
    // question stands.
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&doom) && options.contains(&fracture),
        "\"target enchantment\" reaches both sides of the table — the Aura \
         Fracture is an enchantment too: {options:?}"
    );
    assert!(
        !options.contains(&fodder) && !options.contains(&their_land),
        "a land is no enchantment: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, aura_fracture()).is_some()
            && on_battlefield(&engine, p1, fastbond()).is_some(),
        "and nothing has happened yet: the cost is the last step of the activation"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doom],
            },
        )
        .expect("the enchantment the question offered is the one it destroys");

    // CR 601.2h: the sacrifice is asked only now, the target being settled.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which land, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "the untapped Plains this seat kept back is on the menu: {options:?}"
    );
    assert!(
        options.iter().all(|id| my_lands.contains(id)),
        "every permanent on the menu is a land of this seat's own — the Forest \
         across the table and the Aura Fracture itself are not: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the land the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, plains()).is_some(),
        "the land is in its owner's graveyard the moment the price is paid"
    );
    assert!(
        on_battlefield(&engine, p1, fastbond()).is_some(),
        "and the destruction has not happened yet — it resolves off the stack"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying an enchantment is no mana ability"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, fastbond()).is_some(),
        "the enchantment the ability named is destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, aura_fracture()).is_some(),
        "the enchantment that aimed it was not the target and survived"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, plains()).len(),
        3,
        "exactly one land paid the price"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and nothing else on the other side of the table moved"
    );
}
