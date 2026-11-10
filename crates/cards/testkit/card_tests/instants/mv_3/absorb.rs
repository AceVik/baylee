//! `cards/instants/mv_3/absorb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Absorb — {W}{U}{U} instant: "Counter target spell. You gain 3 life."
///
/// Both printed sentences are read in one exchange, because the counterspell
/// is only itself if the spell it names actually leaves the stack *and* the
/// caster's life total moves. The Elf is the spell being countered: the target
/// question must offer it before any cost is paid (CR 601.2c before
/// CR 601.2h), so the three mana is still floating while that menu stands, and
/// afterwards the card lies in its owner's graveyard, the battlefield is empty,
/// and the three life belongs to the seat that cast the Absorb rather than to
/// the seat whose spell died.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn absorb_counters_the_spell_it_names_and_gains_three_life_for_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[quiet_creature()])
        .battlefield(1, &[plains(), island(), island()])
        .hand(1, &[absorb()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, quiet_creature());
    let elf = on_stack(&engine, quiet_creature()).expect("the Elf is a spell on the stack");
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "a creature spell is not a permanent until it resolves"
    );

    // p0 passes, and p1 answers with priority while that spell is still on the
    // stack — exactly the window an instant is for.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && !stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 20, "nothing gained yet");

    // `LegalActions` is filtered through `can_afford`, which reads the pool and
    // not the untapped lands, so an empty pool is the only difference between
    // the offer below and the one after the tap.
    let spell = in_hand(&engine, p1, absorb()).expect("the Absorb is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the responding seat holds priority, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.castable.contains(&spell),
        "{{W}}{{U}}{{U}} is not three untapped lands: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p1);
    let pool = &engine.state().players[1].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "the Plains");
    assert_eq!(pool.available(ManaColor::Blue), 2, "and the two Islands");
    cast_with_floating(&mut engine, p1, absorb());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast the Absorb names the target");
    assert_eq!((min, max), (1, 1), "one spell, and the spell asks once");
    assert!(
        options.contains(&elf),
        "the spell waiting on the stack is the whole menu: {options:?}"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        3,
        "CR 601.2c before CR 601.2h: the target is named while the mana that \
         pays for it is still floating"
    );

    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the spell the menu offered is the one it counters");
    assert!(
        !stack_is_empty(&engine),
        "and the Absorb itself is what is waiting there now"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_stack(&engine, quiet_creature()).is_none(),
        "the countered spell left the stack"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "and it never becomes the permanent it was cast as"
    );
    assert!(
        in_graveyard(&engine, p1, absorb()).is_some(),
        "the Absorb resolved and is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[1].life,
        23,
        "\"You gain 3 life\" — the caster's life, and exactly three of it"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the seat whose spell died loses nothing"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the {{W}}{{U}}{{U}} came out of the pool"
    );
}
