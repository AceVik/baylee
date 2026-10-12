#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn underdome_commits_a_colored_un_activation_without_changing_the_actual_mana_type() {
    for is_un in [false, true] {
        let mut e = prepared(&[], &[index::UNDERDOME, index::SEWER_RATS], &[]);
        let rat = on_battlefield(&e, P0, index::SEWER_RATS).unwrap();
        print_fact(&mut e, rat, is_un);
        pool(&mut e, P0, &[]);
        restricted(&mut e, P0, ManaColor::Black);
        let life = e.state.players[0].life;
        let action = PlayerAction::ActivateAbility {
            source: rat,
            ability_index: 0,
        };
        if is_un {
            e.apply(P0, action).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state.players[0].life, life - 1);
            assert_eq!(e.state.players[0].mana_pool.total(), 0);
            assert_eq!(
                e.state.object(rat).unwrap().characteristics().power,
                Some(2)
            );
        } else {
            let hash = e.snapshot_hash();
            assert!(e.apply(P0, action).is_err());
            assert_eq!(e.snapshot_hash(), hash);
            assert_eq!(e.state.players[0].life, life);
            assert_eq!(restricted_units(&e, P0), 1);
        }
    }
}
#[test]
fn underdome_pays_only_the_un_imposed_part_of_an_ordinary_activation() {
    for silver_tax in [false, true] {
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
        print_fact(&mut e, staff, false);
        print_fact(&mut e, tax, silver_tax);
        let filter = crate::effects::EffectFilter::object(&e.state, staff);
        impose_on(&mut e, tax, Modifier::AbilitiesCostMore(1), filter);
        e.state.set_tapped(tax, true);
        e.state.set_tapped(staff, true);
        pool(&mut e, P0, &[(ManaColor::Colorless, 1)]);
        restricted(&mut e, P0, ManaColor::Red);
        let action = PlayerAction::ActivateAbility {
            source: staff,
            ability_index: 0,
        };
        if silver_tax {
            e.apply(P0, action).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert!(!is_tapped(&e, staff));
            assert_eq!(e.state.players[0].mana_pool.total(), 0);
        } else {
            let hash = e.snapshot_hash();
            assert!(e.apply(P0, action).is_err());
            assert_eq!(e.snapshot_hash(), hash);
            assert!(is_tapped(&e, staff));
            assert_eq!(restricted_units(&e, P0), 1);
        }
    }
}
#[test]
fn underdome_un_activation_base_does_not_admit_restricted_mana_for_a_black_tax() {
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
    // Produce two genuine restricted units before testing the Staff payment.
    pool(&mut e, P0, &[]);
    restricted(&mut e, P0, ManaColor::Red);
    let land = on_battlefield(&e, P0, index::UNDERDOME).unwrap();
    e.state.set_tapped(land, false);
    e.refresh_offer();
    restricted(&mut e, P0, ManaColor::Red);
    let filter = crate::effects::EffectFilter::object(&e.state, staff);
    impose_on(&mut e, tax, Modifier::AbilitiesCostMore(1), filter);
    e.state.set_tapped(tax, true);
    e.state.set_tapped(staff, true);
    e.refresh_offer();
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: staff,
                ability_index: 0
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), hash);
    assert!(is_tapped(&e, staff));
    assert_eq!(restricted_units(&e, P0), 2);
}
#[test]
fn underdome_pays_an_optional_resolution_cost_only_when_the_imposing_source_is_un() {
    for is_un in [false, true] {
        let mut e = prepared(&[index::RUPTURE_SPIRE], &[index::UNDERDOME], &[]);
        let card = in_hand(&e, P0, index::RUPTURE_SPIRE).unwrap();
        print_fact(&mut e, card, is_un);
        pool(&mut e, P0, &[]);
        restricted(&mut e, P0, ManaColor::Red);
        let spire = play_land(&mut e, P0, index::RUPTURE_SPIRE);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        e.apply(P0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.object(spire).unwrap().zone,
            if is_un {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
        assert_eq!(restricted_units(&e, P0), u32::from(!is_un));
    }
}

#[test]
fn underdome_both_real_mana_abilities_pay_imposed_life_or_reject_atomically() {
    for ability_index in [0, 1] {
        for life in [1, 3] {
            let mut e = prepared(&[], &[index::UNDERDOME, index::SOL_RING], &[]);
            let land = on_battlefield(&e, P0, index::UNDERDOME).unwrap();
            let tax = on_battlefield(&e, P0, index::SOL_RING).unwrap();
            // Narrow imposed-price fixture; no fictional printed rule on Ring.
            let filter = crate::effects::EffectFilter::object(&e.state, land);
            impose_on(&mut e, tax, Modifier::ManaAbilitiesCostLife(2), filter);
            pool(&mut e, P0, &[]);
            e.state.set_tapped(tax, true);
            e.state.players[0].life = life;
            e.refresh_offer();
            let action = PlayerAction::ActivateAbility {
                source: land,
                ability_index,
            };
            if life == 1 {
                let hash = e.snapshot_hash();
                assert!(e.apply(P0, action).is_err());
                assert_eq!(e.snapshot_hash(), hash);
                assert!(!is_tapped(&e, land));
                assert_eq!(e.state.players[0].mana_pool.total(), 0);
                assert_eq!(e.state.players[0].life, life);
            } else {
                e.apply(P0, action).unwrap();
                if ability_index == 1 {
                    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
                        .unwrap();
                }
                assert!(is_tapped(&e, land));
                assert_eq!(e.state.players[0].life, 1);
                assert_eq!(e.state.players[0].mana_pool.total(), 1);
                assert_eq!(restricted_units(&e, P0), ability_index);
                assert!(
                    stack_is_empty(&e),
                    "ordinary mana activation and its life price do not use stack"
                );
            }
        }
    }
}
