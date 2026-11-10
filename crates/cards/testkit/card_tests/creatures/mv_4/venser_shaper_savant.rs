//! `cards/creatures/mv_4/venser_shaper_savant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Venser, Shaper Savant — {2}{U}{U} with flash — prints "When Venser enters,
/// return target spell or permanent to its owner's hand."
///
/// One board plays both halves of that sentence. A creature without flash could
/// not be cast in an opponent's main phase over a non-empty stack at all, so
/// the cast itself is the flash; and the trigger then points at the *spell*
/// still waiting underneath, which no bounce of a permanent can stand in for.
/// The Dark Ritual never resolves — its three black mana never arrive — and the
/// card ends up in its owner's hand rather than in a graveyard, while the
/// trigger's own menu is read for the "or permanent" half beside it.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn venser_bounces_a_spell_off_the_stack_with_flash() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[venser_shaper_savant()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    // p1 taps their Swamp and puts Dark Ritual on the stack. It is the only
    // spell there, and it has no targets, so nothing else can be asked first.
    cast_from_hand(&mut engine, p1, dark_ritual());
    let ritual = on_stack(&engine, dark_ritual()).expect("the Ritual is on the stack");
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "its {{B}} was paid out of the pool, so nothing has been made yet"
    );

    // p0's window: it is p1's turn and the stack is not empty.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let land = on_battlefield(&engine, p0, island()).expect("p0's Islands are out");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands, four blue — {{2}}{{U}}{{U}}"
    );
    let card = in_hand(&engine, p0, venser_shaper_savant()).expect("Venser is in hand");
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the seat holding Venser");
    assert!(
        legal.castable.contains(&card),
        "flash: a creature is castable in an opponent's main phase over a \
         non-empty stack: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, venser_shaper_savant());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{U}}{{U}} came out of the pool the four Islands filled"
    );

    // Venser resolves, and its enters-trigger asks its controller what goes home.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the seat that cast Venser aims the trigger");
    assert_eq!((min, max), (1, 1), "one target, and the trigger asks once");
    assert!(
        options.contains(&ritual),
        "\"target spell or permanent\" — the spell under Venser is on the menu: {options:?}"
    );
    assert!(
        options.contains(&land),
        "and so is a permanent on the battlefield: one menu, both words: {options:?}"
    );
    assert!(
        on_stack(&engine, dark_ritual()).is_some(),
        "nothing has resolved while the question stands, so the spell is still there"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .expect("the spell the question enumerated is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, venser_shaper_savant()).is_some(),
        "Venser stayed on the battlefield: it bounced the spell and not itself"
    );
    assert!(
        on_stack(&engine, dark_ritual()).is_none(),
        "the Ritual left the stack without resolving"
    );
    assert!(
        in_hand(&engine, p1, dark_ritual()).is_some(),
        "\"to its owner's hand\": the bounced spell is in p1's hand, not p0's"
    );
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_none(),
        "and not in a graveyard, which is where a resolved or countered spell goes"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the {{B}}{{B}}{{B}} the Ritual would have made never arrived, so the \
         spell really was bounced rather than allowed to resolve first"
    );
}

/// Venser's "target spell or permanent" over an opponent's trigger: the
/// permanent that put it there is on the menu, and the trigger is not.
///
/// Abilities on the stack aren't spells (CR 113.9). The menu used to offer
/// them, so in l29 game 1930 thirty token Vensers each found every Venser
/// trigger stacked before it on its menu, which grew by one a question
/// until 256 entries wrapped the question's maximum to zero.
#[test]
fn venser_returns_a_spell_or_permanent_and_never_offers_an_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, trigger, bowmasters) =
        an_opponents_trigger_on_the_stack(&[island(); 4], &[venser_shaper_savant()]);

    // Flash: Venser is cast over the trigger and resolves first.
    cast_from_hand(&mut engine, p0, venser_shaper_savant());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "Venser's controller aims its trigger");
    assert!(
        engine.state().zones.contains(trigger, ZoneLocation::Stack),
        "the Bowmasters' trigger is still waiting under it"
    );
    assert!(
        options.contains(&bowmasters),
        "the permanent that put it there is a target: {options:?}"
    );
    assert!(
        !options.contains(&trigger),
        "an ability on the stack is no spell and no permanent: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bowmasters],
                players: vec![],
            },
        )
        .expect("the Bowmasters are a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p1, orcish_bowmasters()).is_some(),
        "the Bowmasters went home to p1's hand"
    );
}
