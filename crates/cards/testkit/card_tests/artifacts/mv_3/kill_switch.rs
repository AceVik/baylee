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

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

fn jayemdae_tome() -> CardIndex {
    card_index("39ee576a-0803-4063-9c84-f2b537e4d44c")
}

#[test]
fn kill_switch_taps_other_artifacts_and_locks_them_while_tapped() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), kill_switch(), sol_ring()])
        .battlefield(1, &[sol_ring()])
        .hand(1, &[jayemdae_tome()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let ks = on_battlefield(&e, P0, kill_switch()).unwrap();
    let p0_ring = on_battlefield(&e, P0, sol_ring()).unwrap();
    let p1_ring = on_battlefield(&e, P1, sol_ring()).unwrap();

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
    assert!(is_tapped(&e, p1_ring));

    // P1 untap step: locked Sol Ring does NOT untap!
    reach_their_main_phase(&mut e, P1);
    assert!(is_tapped(&e, p1_ring));

    // CR 502.3: during P0's untap step Kill Switch untaps, while P0's Sol Ring was still locked as the step began.
    reach_their_main_phase(&mut e, P0);
    assert!(!is_tapped(&e, ks));
    assert!(is_tapped(&e, p0_ring));
    // Now Kill Switch is untapped so the lock has ended: P1's and P0's Sol Rings untap on their next untap steps.
    reach_their_main_phase(&mut e, P1);
    assert!(!is_tapped(&e, p1_ring));
    reach_their_main_phase(&mut e, P0);
    assert!(!is_tapped(&e, p0_ring));
}
