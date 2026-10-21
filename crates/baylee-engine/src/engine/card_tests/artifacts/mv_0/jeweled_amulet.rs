//! `cards/artifacts/mv_0/jeweled_amulet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jeweled_amulet() -> CardIndex {
    card_index("e2ad4b4d-d5fe-4a64-9b40-296c8fc8bcce")
}

/// Jeweled Amulet (e2ad4b4d-d5fe-4a64-9b40-296c8fc8bcce) — {0} Artifact:
/// - "{1}, {T}: Put a charge counter on this artifact. Note the type of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact."
#[test]
fn jeweled_amulet_puts_charge_counter_only_when_it_has_no_charge_counters() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[jeweled_amulet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Cast {0} Jeweled Amulet from hand.
    cast_from_hand(&mut engine, P0, jeweled_amulet());
    pass_until(&mut engine, stack_is_empty);

    let amulet = on_battlefield(&engine, P0, jeweled_amulet()).expect("Amulet entered");
    assert_eq!(
        engine
            .state()
            .object(amulet)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );
    assert!(
        ability_offered(&engine, amulet, 0),
        "offered when there are 0 charge counters on Jeweled Amulet"
    );

    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, jeweled_amulet(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, amulet));
    assert_eq!(
        engine
            .state()
            .object(amulet)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1,
        "1 charge counter placed on Jeweled Amulet"
    );

    // Next turn cycle: Jeweled Amulet untaps, but since it has 1 charge counter,
    // the ability ("Activate only if there are no charge counters on this artifact") is not offered.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(!is_tapped(&engine, amulet));
    assert!(
        !ability_offered(&engine, amulet, 0),
        "not offered while Jeweled Amulet already has a charge counter"
    );
}
