//! `cards/artifacts/mv_3/kill_switch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kill_switch() -> CardIndex {
    card_index("0cd5eef6-6830-4c6d-8578-2726900c39a5")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

fn time_warp() -> CardIndex {
    card_index("dbd6a94b-62ff-4a10-9d52-bdd90b26e425")
}

#[test]
fn kill_switch_taps_other_artifacts_and_locks_only_existing_ones_while_tapped() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), kill_switch(), sol_ring()])
        .battlefield(
            1,
            &[
                sol_ring(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(1, &[sol_ring(), time_warp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let ks = on_battlefield(&e, P0, kill_switch()).unwrap();
    let p0_ring = on_battlefield(&e, P0, sol_ring()).unwrap();
    let p1_old_ring = on_battlefield(&e, P1, sol_ring()).unwrap();

    let forests: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            e.state()
                .object(id)
                .is_some_and(|o| o.controller == P0 && o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: forests[0],
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: forests[1],
            ability_index: 0,
        },
    )
    .unwrap();
    activate(&mut e, P0, kill_switch(), 0);
    pass_until(&mut e, stack_is_empty);

    assert!(is_tapped(&e, ks));
    assert!(is_tapped(&e, p0_ring));
    assert!(is_tapped(&e, p1_old_ring));

    reach_their_main_phase(&mut e, P1);
    assert!(is_tapped(&e, p1_old_ring));

    // P1 casts a new Sol Ring after Kill Switch resolved, taps it, and takes an extra turn while Kill Switch stays tapped.
    cast_from_hand(&mut e, P1, sol_ring());
    pass_until(&mut e, stack_is_empty);
    let p1_new_ring = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            id != p1_old_ring
                && e.state().object(id).is_some_and(|o| {
                    o.controller == P1 && o.card.is_some_and(|c| c.index == sol_ring())
                })
        })
        .unwrap();
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: p1_new_ring,
            ability_index: 0,
        },
    )
    .unwrap();
    assert!(is_tapped(&e, p1_new_ring));
    let turn_before = e.state().turn.number;
    cast_from_hand(&mut e, P1, time_warp());
    e.apply(P1, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut e, stack_is_empty);

    pass_until(&mut e, |e| e.state().turn.number > turn_before);
    reach_their_main_phase(&mut e, P1);
    assert!(is_tapped(&e, ks));
    assert!(is_tapped(&e, p1_old_ring), "locked artifact stays tapped");
    assert!(
        !is_tapped(&e, p1_new_ring),
        "artifact entering after resolution untaps (CR 611.2c)"
    );

    // CR 502.3: during P0 untap step Kill Switch untaps while P0 Sol Ring was still locked at step start.
    reach_their_main_phase(&mut e, P0);
    assert!(!is_tapped(&e, ks));
    assert!(is_tapped(&e, p0_ring));
    reach_their_main_phase(&mut e, P1);
    assert!(!is_tapped(&e, p1_old_ring));
    reach_their_main_phase(&mut e, P0);
    assert!(!is_tapped(&e, p0_ring));
}
