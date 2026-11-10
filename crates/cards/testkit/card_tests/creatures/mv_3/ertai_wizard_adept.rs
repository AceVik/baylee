//! `cards/creatures/mv_3/ertai_wizard_adept.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ertai, Wizard Adept is a legendary 1/1 whose whole text is
/// "`{2}{U}{U}`, `{T}`: Counter target spell." — a card that only exists at
/// the moment somebody else's spell is on the stack. The scenario builds
/// exactly that: seven untapped Islands and the Wizard already in play, and
/// p1 casting Dark Ritual — a spell with no target whose resolution would be
/// legible as three black mana in p1's pool — inside p0's own main phase, so
/// the mana p0 floats survives the pass. The offer is read on both sides of
/// the tap for the same reason `claws_of_gix` reads it twice: with an empty
/// pool the ability is *absent* from `LegalActions::abilities` (`can_afford`
/// looks at the pool, not at untapped lands) and it appears the moment four
/// of the seven float, which is what says the four mana are a price and not
/// a label. The tap and the {2}{U}{U} are asserted after the target is
/// answered, because CR 601.2c picks the target and CR 601.2h pays last, and
/// what the card does is then the only thing left to see: Dark Ritual in its
/// owner's graveyard, its black mana never arriving, and Ertai still standing.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn ertai_wizard_adept_counters_a_spell_for_four_mana_and_its_own_tap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                ertai_wizard_adept(),
            ],
        )
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ertai = on_battlefield(&engine, p0, ertai_wizard_adept()).expect("Ertai is out");
    assert!(!is_tapped(&engine, ertai), "nothing has tapped him yet");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing of p0's is floating"
    );

    // Read before a single mana is floating: `can_afford` reads the pool, so
    // an ability whose price is four mana is simply not on the list.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ertai, 0)),
        "{{2}}{{U}}{{U}} off an empty pool is unpayable, and an ability nobody \
         can afford is left out of the offer rather than refused: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Islands and no other mana source on this board"
    );
    // And with the money on the table it is *still* not offered, which is
    // the sharper half: "counter target spell" is the only thing this card
    // does, the stack is empty, and an ability with no legal target is left
    // out of the offer exactly as an unaffordable one is (CR 115.2 — only a
    // spell is a legal target here, and there is no spell). Without this
    // reading the assertion above would have been satisfied by either
    // reason, and the one below by neither.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ertai, 0)),
        "paid for and aimed at nothing: {:?}",
        legal.abilities
    );

    // The spell to counter has to come from somebody else, so p0 hands
    // priority over inside the same main phase — a pool is not emptied until
    // the step or phase ends (CR 500.5).
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, dark_ritual());
    let spell = on_stack(&engine, dark_ritual()).expect("Dark Ritual is on the stack");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    // Now both halves of the price are met and the target exists, so the one
    // line the card prints is on the list.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ertai, 0)),
        "with four of the seven floating and a spell to aim at, the line is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, ertai_wizard_adept(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"counter target spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one spell");
    assert!(
        options.contains(&spell),
        "the spell on the stack is the whole of what may be countered: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and it is the only spell in the game: {options:?}"
    );
    assert!(
        on_stack(&engine, dark_ritual()).is_some(),
        "the target is chosen before the costs are paid (CR 601.2c, then CR 601.2h)"
    );
    assert!(
        !is_tapped(&engine, ertai),
        "so neither half of the price has been paid yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("the spell the question offered is the one that answers it");

    assert!(
        is_tapped(&engine, ertai),
        "{{T}} is half the printed price and the Wizard paid it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and {{2}}{{U}}{{U}} came out of the seven"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "a countered spell is put into its owner's graveyard (CR 701.6a)"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "and the Ritual that never resolved added none of its three black mana"
    );
    assert!(
        on_battlefield(&engine, p0, ertai_wizard_adept()).is_some(),
        "the Wizard outlives the spell he countered"
    );
}
