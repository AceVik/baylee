//! `cards/creatures/mv_2/goblin_firebug.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Goblin Firebug (`ee337677-d898-40a5-9453-405d07ec12b1`) — {1}{R} Creature — Goblin 2/2:
/// "When this creature leaves the battlefield, sacrifice a land."
#[test]
fn goblin_firebug_forces_land_sacrifice_when_it_leaves_battlefield() {
    let firebug_card = card_index("ee337677-d898-40a5-9453-405d07ec12b1");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[firebug_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, firebug_card);
    pass_until(&mut engine, stack_is_empty);

    let firebug = on_battlefield(&engine, p0, firebug_card).unwrap();
    let land = on_battlefield(&engine, p0, mountain()).unwrap();
    bury(&mut engine, &[firebug]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, firebug_card).is_some());
    assert!(in_graveyard(&engine, p0, mountain()).is_some());
}
