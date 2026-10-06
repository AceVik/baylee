//! `cards/artifacts/mv_4/tower_of_eons.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tower of Eons prints one line — "{8}, {T}: You gain 10 life." — and every
/// part of that price has to be played to be believed. Twelve Forests are
/// exactly the {4} that brings the artifact to the table plus the {8} its
/// ability charges, so the pool reads twelve, then eight, then nothing: an
/// activation that had skipped its generic cost would leave the mana behind,
/// and one that had skipped its {T} would leave the artifact standing. The life
/// is read on both seats, because "you gain" is the controller's word and not
/// the table's, and the same board a turn later — untapped, empty pool — is the
/// control for the offer the eight mana bought.
#[test]
fn tower_of_eons_taps_and_eight_mana_for_ten_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 12])
        .hand(0, &[tower_of_eons()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Twelve Forests into the pool first: the {4} the artifact costs and the
    // {8} its ability charges are one payment, and CR 500.5 keeps what is left
    // in the pool because the whole scenario stays inside this one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "twelve Forests tapped, and the Tower makes no mana of its own"
    );
    cast_with_floating(&mut engine, p0, tower_of_eons());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let tower = on_battlefield(&engine, p0, tower_of_eons()).expect("the Tower resolved");
    assert!(!is_tapped(&engine, tower), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cast's {{4}} is spent and exactly the {{8}} the ability charges is left"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // and not the untapped lands — so the claim is made with the mana already
    // floating, where the engine reads it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tower, 0)),
        "the one line the card prints, now that its {{8}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, tower_of_eons(), 0);
    assert!(
        is_tapped(&engine, tower),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the other half was the eight mana that was floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "gaining life is no mana ability, so the ability is on the stack"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing has been gained while it is still waiting there"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 30, "\"You gain 10 life.\"");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that paid the price, not to the opponent"
    );
    assert!(
        on_battlefield(&engine, p0, tower_of_eons()).is_some(),
        "the cost was the tap and the mana, so the artifact is still standing"
    );

    // The control for the offer above: the same board a turn later, where the
    // untap step has stood every Forest and the Tower back up but the pool
    // emptied with the step that ended (CR 500.5).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, tower),
        "the untap step stood the Tower back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(tower, 0)),
        "{{8}} is not eight untapped Forests: with nothing floating the cost is \
         unpayable and nothing is offered: {:?}",
        legal.abilities
    );
}
