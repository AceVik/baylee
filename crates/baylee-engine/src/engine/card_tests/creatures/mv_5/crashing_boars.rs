//! `cards/creatures/mv_5/crashing_boars.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn crashing_boars() -> CardIndex {
    card_index("a4c59feb-e76a-4e08-b296-21c3e61cfc64")
}

/// Crashing Boars (`a4c59feb-e76a-4e08-b296-21c3e61cfc64`) — {3}{G}{G} Creature — Boar (Partial):
/// "Whenever this creature attacks, defending player chooses an untapped creature they control. That creature blocks this creature this turn if able."
#[test]
fn crashing_boars_casts_and_enters_as_4_4() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[crashing_boars()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    tap_all_mana(&mut engine, P0);
    cast_from_hand(&mut engine, P0, crashing_boars());
    pass_until(&mut engine, stack_is_empty);

    let boars = on_battlefield(&engine, P0, crashing_boars()).unwrap();
    assert_eq!(pt(&engine, boars), (4, 4));
}
