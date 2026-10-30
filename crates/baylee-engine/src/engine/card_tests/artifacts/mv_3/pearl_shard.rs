//! Pearl Shard's delayed prevention shield, played.

use super::*;

fn pearl_shard() -> CardIndex {
    card_index("d08cb402-2138-4e8e-99f4-4a62855dc285")
}

#[test]
fn pearl_shard_prevents_exactly_the_next_two_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[pearl_shard(), plains(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, pearl_shard(), 1);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    cast_from_hand(&mut e, p0, lightning_bolt());
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 19, "only the third point lands");
}
