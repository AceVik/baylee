//! `cards/enchantments/mv_3/phyrexian_tyranny.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Phyrexian Tyranny — {U}{B}{R} Enchantment
/// "Whenever a player draws a card, that player loses 2 life unless they pay {2}."
#[test]
fn phyrexian_tyranny_makes_player_lose_two_life_when_drawing_without_paying_two() {
    let tyranny = card_index("55082cd0-a33c-4f1d-9f46-9578f63e6bb6");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), swamp(), mountain()])
        .hand(0, &[tyranny])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, tyranny);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, tyranny).is_some());

    // Advance to P1's main phase; P1 draws a card in their draw step with no lands to pay {2}.
    assert!(walk_to_own_main(&mut engine, p1));
    assert_eq!(engine.state().players[1].life, 18);
}

#[test]
fn phyrexian_tyranny_spares_player_who_pays_two_mana_on_draw() {
    let tyranny = card_index("55082cd0-a33c-4f1d-9f46-9578f63e6bb6");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[tyranny])
        .battlefield(1, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Advance into P1's draw step while Phyrexian Tyranny's trigger is on the stack;
    // P1 floats {U}{U} and pays {2} when the trigger resolves.
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    tap_all_mana(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    assert!(walk_to_own_main(&mut engine, p1));
    assert_eq!(engine.state().players[1].life, 20);
}
