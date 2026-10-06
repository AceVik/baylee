//! `cards/instants/mv_2/hisoka_s_defiance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hisoka's Defiance prints one sentence — "Counter target Spirit or Arcane
/// spell" — so the card is only itself when the *filter* decides what may be
/// aimed at, and the scenario plays both sides of it in one main phase. A Kor
/// Spirit spell is countered while it is still a spell; an Elf spell cast
/// afterwards is the control, and p1's second copy is read while the same
/// {1}{U} is floating, so where the offer disappears it disappears for the
/// subtype and for nothing else.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn hisokas_defiance_counters_a_spirit_spell_and_declines_an_elf() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[plains(), plains(), forest(), forest()])
        .hand(0, &[skyclave_apparition(), llanowar_elves()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[hisoka_s_defiance(), hisoka_s_defiance()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{W}{W} off both Plains and one Forest. The other Forest is named as
    // the source kept back: it is what pays for the Elf in the second half,
    // and tapping it here would leave that cast with nothing behind it.
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(
        forests.len(),
        2,
        "a Forest to pay with and a Forest to keep"
    );
    tap_mana_except(&mut engine, p0, forests[1]);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Plains and one Forest is exactly the {{1}}{{W}}{{W}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spirit = in_hand(&engine, p0, skyclave_apparition()).expect("the Spirit is in hand");
    assert!(
        legal.castable.contains(&spirit),
        "the three mana in the pool pay for it: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, skyclave_apparition());

    // p1 gets priority while the Spirit spell is still on the stack: a spell
    // is countered while it is a spell, and once it resolves there is nothing
    // left to point at.
    pass_until(&mut engine, |e| {
        on_stack(e, skyclave_apparition()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    let apparition_spell =
        on_stack(&engine, skyclave_apparition()).expect("the Spirit spell is on the stack");

    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        4,
        "four Islands, four blue"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "and it is p1 being asked");
    let defiance = in_hand(&engine, p1, hisoka_s_defiance()).expect("the Defiance is in hand");
    assert!(
        legal.castable.contains(&defiance),
        "a Spirit spell on the stack and {{1}}{{U}} floating is a castable \
         Hisoka's Defiance: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p1, hisoka_s_defiance());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Spirit or Arcane spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the casting seat aims it");
    assert_eq!((min, max), (1, 1), "one spell, no more and no fewer");
    assert!(
        options.contains(&apparition_spell),
        "the Kor Spirit is one of the options: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and the only spell on the stack, so the filter is the whole menu: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![apparition_spell],
            },
        )
        .expect("the spell the question offered was one of its options");

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        in_graveyard(&engine, p0, skyclave_apparition()).is_some(),
        "a countered spell goes to its owner's graveyard, not to the counter's"
    );
    assert!(
        on_battlefield(&engine, p0, skyclave_apparition()).is_none(),
        "and the Spirit never arrived, so its enters-trigger never fired"
    );
    assert!(
        in_graveyard(&engine, p1, hisoka_s_defiance()).is_some(),
        "the Defiance itself resolved and went to the graveyard"
    );

    // The control, and the half of the printed sentence that is a filter. The
    // Elf spell is no Spirit and no Arcane spell, and p1's pool still holds
    // the second {1}{U} out of the four Islands — so the price is no reason to
    // decline, and a Defiance that had stopped reading the subtype would be
    // offered here.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("p1 holds priority again, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "and it is p1 being asked");
    let second = in_hand(&engine, p1, hisoka_s_defiance()).expect("the second copy is in hand");
    assert!(
        on_stack(&engine, llanowar_elves()).is_some(),
        "the Elf spell is on the stack, which is where a counter would aim"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "and the second {{1}}{{U}} is still floating, so the price is no \
         reason to decline"
    );
    assert!(
        !legal.castable.contains(&second),
        "an Elf spell is neither a Spirit nor an Arcane spell, so the second \
         Defiance has nothing it may be pointed at: {:?}",
        legal.castable
    );
}
