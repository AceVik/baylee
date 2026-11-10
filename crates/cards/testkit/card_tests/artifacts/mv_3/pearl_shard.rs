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

/// The `{3}, {T}` half prevents the same two damage as the `{W}` half.
#[test]
fn pearl_shard_generic_ability_prevents_two_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[pearl_shard(), island(), island(), island(), mountain()],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let mountain_id = on_battlefield(&e, p0, mountain()).unwrap();
    tap_mana_except(&mut e, p0, mountain_id);
    activate(&mut e, p0, pearl_shard(), 0);
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
    assert_eq!(e.state().players[1].life, 19);
}

/// "This turn": a shield left unused on p0's turn is gone by p0's next
/// turn, and the Bolt then deals all three.
#[test]
fn pearl_shard_prevention_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[pearl_shard(), plains(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let mountain_id = on_battlefield(&e, p0, mountain()).unwrap();
    tap_mana_except(&mut e, p0, mountain_id);
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
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
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
    assert_eq!(e.state().players[1].life, 17, "the shield expired");
}
