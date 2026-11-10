//! `cards/lands/horizon/silent_clearing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Silent Clearing prints two lines and nothing else: "{T}, Pay 1 life: Add
/// {W} or {B}." and "{1}, {T}, Sacrifice this land: Draw a card." Both are
/// played in one main phase off two copies, because each is the other's
/// price — the first land pays the one life and the one mana the second
/// charges, so the life total, the pool, the battlefield and the graveyard
/// have to agree with every clause of the card at once. The colour question
/// is the half no count can see: "or" is a real choice, so the black mana in
/// the pool is only there because the answer was read, and a land that added
/// white regardless would satisfy every other assertion here. The tapped copy
/// is also the clean control for `{T}` being part of the second line's price,
/// with the mana for it already floating.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn silent_clearing_trades_life_for_colored_mana_and_trades_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[silent_clearing(), silent_clearing()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let clearings = all_on_battlefield(&engine, p0, silent_clearing());
    assert_eq!(clearings.len(), 2, "both copies are on the table");
    let life_before = engine.state().players[0].life;
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before a land is tapped"
    );

    // Ability 0: "{T}, Pay 1 life: Add {W} or {B}." A mana ability whose
    // price is *not* its own tap, so `tap_all_mana` leaves it alone and it is
    // pressed by index.
    activate(&mut engine, p0, silent_clearing(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{W}} or {{B}}` is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated names the colour");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "the two colours the card prints: {options:?}"
    );
    assert_eq!(options.len(), 2, "`or` offers no third: {options:?}");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours the ability offered");

    assert_eq!(
        engine.state().players[0].life,
        life_before - 1,
        "\"Pay 1 life\" is a cost, paid as the ability is activated"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, and the life was the other price"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );

    let spent = clearings
        .iter()
        .copied()
        .find(|id| is_tapped(&engine, *id))
        .expect("the land that paid its own {T}");
    let other = clearings
        .iter()
        .copied()
        .find(|id| *id != spent)
        .expect("the copy that did not tap");
    assert!(!is_tapped(&engine, other), "and only one of them tapped");

    // Ability 1: "{1}, {T}, Sacrifice this land: Draw a card." The mana the
    // neighbour just made is exactly the {1}, so the whole price comes off
    // the board this test built.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(other, 1)),
        "the untapped copy's second line is affordable with the {{1}} floating: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(spent, 1)),
        "and the tapped one's is not — the pool covers the {{1}} either way, \
         so {{T}} is the whole of the difference: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, silent_clearing(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p0, silent_clearing()),
        vec![spent],
        "the untapped copy sacrificed itself and the tapped one stayed"
    );
    assert!(
        in_graveyard(&engine, p0, silent_clearing()).is_some(),
        "a sacrificed land goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}} it charged came out of the pool"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\""
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "the card arrived in hand, not merely off the library"
    );
}
