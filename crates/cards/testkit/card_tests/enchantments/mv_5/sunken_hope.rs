//! `cards/enchantments/mv_5/sunken_hope.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Sunken Hope — {3}{U}{U} Enchantment
/// "At the beginning of each player's upkeep, that player returns a creature they control to its owner's hand."
#[test]
fn sunken_hope_forces_active_player_to_return_a_creature_on_upkeep() {
    let hope = card_index("1820b6e9-83fd-40b9-953e-ba1a8ae69614");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[hope, llanowar_elves()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    let p0_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Llanowar Elves on board");
    let p1_bears = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Grizzly Bears on board");

    // Turn 1 upkeep (P0): Sunken Hope triggers and asks P0 to return their creature.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_elf],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(in_hand(&engine, p0, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());

    // Turn 2 upkeep (P1): Sunken Hope triggers and asks P1 to return their creature.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(in_hand(&engine, p1, grizzly_bears()).is_some());
}
