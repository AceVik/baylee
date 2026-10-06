//! `cards/enchantments/mv_3/teferi_s_care.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Teferi's Care prints two lines and both of them are filters: "{W},
/// Sacrifice an enchantment: Destroy target enchantment" reads *you control* at
/// its price and the whole table at its target, while "{3}{U}{U}: Counter
/// target enchantment spell" reads the stack.
///
/// So the same enchantment stands on both sides of the table and a Dark Ritual
/// waits on the stack beside the enchantment spell: the sacrifice menu holds
/// this seat's two enchantments while the destroy menu holds all three, and the
/// counter's menu holds the enchantment spell alone. Both lines are played
/// inside one main phase off eight basics, so every printed price is counted in
/// the pool rather than assumed (CR 500.5).
#[test]
#[allow(clippy::too_many_lines)] // one card, both its lines, every price read off a zone
fn teferis_care_sacrifices_an_enchantment_to_destroy_one_and_counters_an_enchantment_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                teferis_care(),
                exploration(),
                plains(),
                swamp(),
                forest(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[exploration(), dark_ritual()])
        .battlefield(1, &[exploration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let care = on_battlefield(&engine, p0, teferis_care()).expect("the Care is on the table");
    let my_chant = on_battlefield(&engine, p0, exploration()).expect("my enchantment is out");
    let their_chant = on_battlefield(&engine, p1, exploration()).expect("their enchantment is out");

    // One main phase pays for everything: eight basics make {W} for the first
    // line, {G} and {B} for the two spells the second line is about, and
    // {3}{U}{U} to counter one of them. CR 500.5 keeps the pool across all of
    // it, so this is one payment and not three.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight basics tapped, and neither the Care nor the Explorations make mana"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(care, 0)),
        "with the white floating the first line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, teferis_care(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "one enchantment, and the ability asks once"
    );
    assert!(
        options.contains(&my_chant) && options.contains(&care) && options.contains(&their_chant),
        "\"target enchantment\" names any enchantment on either side of the \
         table: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "the three enchantments on the battlefield and nothing else: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // {W} is still floating and the enchantment that will pay it is still
    // standing while this question is open.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cost is the last step of the activation, so nothing is spent yet"
    );
    assert!(
        on_battlefield(&engine, p0, exploration()).is_some(),
        "and nothing has been sacrificed yet, for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_chant],
            },
        )
        .expect("the enchantment across the table was one of the options");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one enchantment, no more and no fewer");
    assert!(
        options.contains(&my_chant),
        "the enchantment you control is on the menu: {options:?}"
    );
    assert!(
        options.contains(&care),
        "\"an enchantment\" is not \"another\": the Care is itself an \
         enchantment this seat controls, so it may pay its own price: {options:?}"
    );
    assert!(
        !options.contains(&their_chant),
        "`CR 701.21a`: an opponent's enchantment is not yours to sacrifice: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_chant],
            },
        )
        .expect("the enchantment the question offered pays the cost");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "the {{W}} came out of the pool"
    );
    assert_eq!(
        mine(&engine, p0, exploration(), Zone::Graveyard).len(),
        1,
        "and the enchantment that was named is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, exploration()).is_none(),
        "nothing has been destroyed yet: the ability is still on the stack"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying is no mana ability, so the ability waits to resolve"
    );

    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        in_graveyard(&engine, p1, exploration()).is_some(),
        "\"destroy target enchantment\" — the enchantment across the table is gone"
    );
    assert!(
        on_battlefield(&engine, p1, exploration()).is_none(),
        "and it left the battlefield, which is what destroy means"
    );
    assert!(
        on_battlefield(&engine, p0, teferis_care()).is_some(),
        "the Care ate another enchantment and is still standing"
    );

    // The second line wants a spell to point at, so the other copy of the same
    // enchantment is cast and priority held over it: a countered spell and a
    // resolved one differ only in the question that gets asked about them.
    cast_with_floating(&mut engine, p0, exploration());
    pass_until(&mut engine, |e| {
        on_stack(e, exploration()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let chant_spell = on_stack(&engine, exploration()).expect("the other copy is a spell now");

    // A spell that is no enchantment, so the menu below has something it must
    // decline. Dark Ritual asks nothing of its own while it waits.
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, |e| {
        on_stack(e, exploration()).is_some()
            && on_stack(e, dark_ritual()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let ritual_spell = on_stack(&engine, dark_ritual()).expect("the Ritual is on the stack");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the green and the black paid for the two spells, and five blue is left \
         — exactly the counter's {{3}}{{U}}{{U}}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        5,
        "and every land left standing on this board makes blue"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(care, 1)),
        "with five blue floating the counter is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, teferis_care(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "one spell, and the ability asks once");
    assert!(
        options.contains(&chant_spell),
        "the enchantment spell waiting on the stack is the target: {options:?}"
    );
    assert!(
        !options.contains(&ritual_spell),
        "the Ritual is a spell and no enchantment spell, so it is off the \
         menu: {options:?}"
    );
    assert!(
        !options.contains(&care),
        "the Care is a permanent on the battlefield and no spell at all: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and that one spell is the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chant_spell],
            },
        )
        .expect("the spell the question offered is the one that is countered");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{U}}{{U}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "countering is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        mine(&engine, p0, exploration(), Zone::Graveyard).len(),
        2,
        "a countered spell goes to its owner's graveyard: the copy that was \
         cast and countered lies beside the one that was sacrificed"
    );
    assert!(
        all_on_battlefield(&engine, p0, exploration()).is_empty(),
        "and no Exploration of mine ever reached the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_some(),
        "the spell the counter did not name resolved instead of being countered"
    );
    assert!(
        on_battlefield(&engine, p0, teferis_care()).is_some(),
        "the Care is still standing after both of its printed lines"
    );
}
