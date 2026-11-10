//! `cards/creatures/mv_3/morgue_toad.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Morgue Toad is a `{2}{B}` 2/2 whose only line is a *mana* ability paid
/// with the creature itself: "Sacrifice this creature: Add {U}{R}"
/// (CR 605.1a). The three Swamps are spent to the last drop by the cast, so
/// the blue and the red that appear afterwards have no other source on the
/// board — and the Llanowar Elves standing beside it are the control for
/// "this creature": the price names the source, so nothing is asked on the
/// way and the Elf is still untapped when the Toad is already in its owner's
/// graveyard. The empty stack is the third reading: a mana ability resolves
/// as it is activated (CR 605.3b), so the mana and the card's departure are
/// one step and not two.
#[test]
fn morgue_toad_sacrifices_itself_for_blue_and_red_and_asks_nothing_on_the_way() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), quiet_creature()])
        .hand(0, &[morgue_toad()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{B} out of the three Swamps, and the Elf named as the printing kept
    // back: `tap_all_mana` would have drunk its own `{T}: Add {G}` and made
    // every count below a claim about a fourth mana.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Swamps tapped, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, morgue_toad());
    pass_until(&mut engine, stack_is_empty);

    let toad = on_battlefield(&engine, p0, morgue_toad()).expect("the Toad resolved");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    assert_eq!(pt(&engine, toad), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast spent all three Swamps, so nothing this board can produce \
         stands behind the two mana read below"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(toad, 0)),
        "a mana ability the card prints is an ordinary indexed entry in \
         `abilities` (CR 605.1a), and its only price is the creature: {:?}",
        legal.abilities
    );

    // By hand and not through `tap_all_mana`, which presses only what costs
    // its own `{T}`: this ability costs the permanent outright.
    activate(&mut engine, p0, morgue_toad(), 0);

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and nothing was asked on the way — the price names the source itself, \
         so this is no `ChooseCards` question about which creature, got {:?}",
        engine.pending()
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "Add {{U}}");
    assert_eq!(pool.available(ManaColor::Red), 1, "and Add {{R}}");
    assert_eq!(
        pool.total(),
        2,
        "two mana out of one sacrifice, and the only source still standing on \
         this board made neither of them"
    );
    assert!(
        on_battlefield(&engine, p0, morgue_toad()).is_none(),
        "sacrificing itself is the whole price, so the Toad is gone"
    );
    assert!(
        in_graveyard(&engine, p0, morgue_toad()).is_some(),
        "and a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "the Elf was never on a menu: the price is *this* creature and not \
         \"a creature\""
    );
    assert!(
        !is_tapped(&engine, elf),
        "which is also why it is still standing untapped — the price was no tap"
    );
}
