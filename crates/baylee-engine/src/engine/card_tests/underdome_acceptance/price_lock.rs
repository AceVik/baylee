//! Child of `underdome_acceptance`. Real actions; explicit price/print fixtures.
#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn underdome_activation_locks_un_tax_quota_when_real_lotus_source_is_sacrificed_for_payment() {
    let mut e = prepared(
        &[],
        &[
            index::UNDERDOME,
            index::CANDELABRA_OF_TAWNOS,
            index::BLACK_LOTUS,
            index::FOREST,
        ],
        &[],
    );
    let artifact = on_battlefield(&e, P0, index::CANDELABRA_OF_TAWNOS).unwrap();
    let lotus = on_battlefield(&e, P0, index::BLACK_LOTUS).unwrap();
    let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
    print_fact(&mut e, artifact, false);
    print_fact(&mut e, lotus, true);
    let filter = crate::effects::EffectFilter::object(&e.state, artifact);
    impose_on(&mut e, lotus, Modifier::AbilitiesCostMore(1), filter);
    pool(&mut e, P0, &[]);
    restricted(&mut e, P0, ManaColor::Red);
    e.state.set_tapped(forest, true);
    e.refresh_offer();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: artifact,
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        targets(&mut e, vec![forest], vec![]);
    }
    assert!(
        matches!(e.pending(), Pending::Priority { .. }),
        "actual activation mana opportunity"
    );
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: lotus,
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(e.state.object(lotus).unwrap().zone, Zone::Graveyard);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        targets(&mut e, vec![forest], vec![]);
    }
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, forest));
    assert_eq!(
        restricted_units(&e, P0),
        0,
        "locked Un tax still admits genuine red credit after Lotus leaves"
    );
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Blue),
        2,
        "locked X1+tax1 price spends one ordinary Lotus unit and one eligible credit"
    );
}
#[test]
fn underdome_un_staff_actual_cost_reduction_uses_floor_not_nominal_amount() {
    for minimum in [1, 2] {
        let mut e = prepared(
            &[],
            &[
                index::UNDERDOME,
                index::STAFF_OF_DOMINATION,
                index::BLACK_LOTUS,
            ],
            &[],
        );
        let staff = on_battlefield(&e, P0, index::STAFF_OF_DOMINATION).unwrap();
        let tax = on_battlefield(&e, P0, index::BLACK_LOTUS).unwrap();
        print_fact(&mut e, staff, true);
        print_fact(&mut e, tax, false);
        let filter = crate::effects::EffectFilter::object(&e.state, staff);
        impose_on(&mut e, tax, Modifier::AbilitiesCostMore(1), filter);
        // Price-only fixture isolates the actual minimum; real Staff untap
        // activation/base1 and real Underdome mana remain unmodified.
        impose_on(
            &mut e,
            tax,
            Modifier::AbilitiesCostLess { amount: 2, minimum },
            filter,
        );
        pool(&mut e, P0, &[]);
        restricted(&mut e, P0, ManaColor::Red);
        let land = on_battlefield(&e, P0, index::UNDERDOME).unwrap();
        e.state.set_tapped(land, false);
        e.refresh_offer();
        restricted(&mut e, P0, ManaColor::Red);
        e.state.set_tapped(tax, true);
        e.state.set_tapped(staff, true);
        e.refresh_offer();
        let action = PlayerAction::ActivateAbility {
            source: staff,
            ability_index: 0,
        };
        if minimum == 1 {
            e.apply(P0, action).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert!(!is_tapped(&e, staff));
            assert_eq!(restricted_units(&e, P0), 1);
        } else {
            let before = e.snapshot_hash();
            assert!(e.apply(P0, action).is_err());
            assert_eq!(e.snapshot_hash(), before);
            assert!(is_tapped(&e, staff));
            assert_eq!(restricted_units(&e, P0), 2);
        }
    }
}
