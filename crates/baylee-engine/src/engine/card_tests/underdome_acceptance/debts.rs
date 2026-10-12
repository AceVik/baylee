#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn underdome_pays_a_source_linked_asp_debt_after_actual_source_destruction() {
    for is_un in [false, true] {
        let mut e = prepared(&[index::TERROR], &[index::NAFS_ASP], &[index::UNDERDOME]);
        let asp = on_battlefield(&e, P0, index::NAFS_ASP).unwrap();
        print_fact(&mut e, asp, is_un);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
        });
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(asp, Defender::Player(P1))],
            },
        )
        .unwrap();
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .unwrap();
        pass_until(&mut e, |e| {
            e.state.turn.step == crate::turn::Step::CombatEnd && stack_is_empty(e)
        });
        // Destroy the actual source with a real spell after its trigger created
        // the debt. The source's physical facts must remain available via LKI.
        priority(&mut e, P0);
        pool(
            &mut e,
            P0,
            &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        );
        let terror = in_hand(&e, P0, index::TERROR).unwrap();
        e.apply(P0, PlayerAction::CastSpell { card: terror })
            .unwrap();
        targets(&mut e, vec![asp], vec![]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.object(asp).unwrap().zone, Zone::Graveyard);
        pass_until(&mut e, |e| {
            e.state.turn.active == P1
                && e.state.turn.step == crate::turn::Step::Upkeep
                && matches!(e.pending(), Pending::Priority { player: P1, .. })
        });
        pool(&mut e, P1, &[]);
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("debt payment priority")
        };
        let debt = legal
            .granted_actions
            .iter()
            .find(|o| {
                matches!(
                    o.effect,
                    crate::choice::GrantedActionKind::CancelDelayedLifeLoss { amount: 1 }
                )
            })
            .expect("independent debt while mana sources remain available")
            .id;
        restricted(&mut e, P1, ManaColor::Red);
        let life = e.state.players[1].life;
        if is_un {
            e.apply(P1, PlayerAction::TakeGrantedAction { id: debt })
                .unwrap();
            assert_eq!(e.state.players[1].mana_pool.total(), 0);
            assert!(stack_is_empty(&e));
            let hash = e.snapshot_hash();
            assert!(
                e.apply(P1, PlayerAction::TakeGrantedAction { id: debt })
                    .is_err()
            );
            assert_eq!(e.snapshot_hash(), hash);
        } else {
            let hash = e.snapshot_hash();
            assert!(
                e.apply(P1, PlayerAction::TakeGrantedAction { id: debt })
                    .is_err()
            );
            assert_eq!(e.snapshot_hash(), hash);
            assert_eq!(restricted_units(&e, P1), 1);
        }
        pass_until(&mut e, |e| {
            e.state.turn.active == P1
                && e.state.turn.step == crate::turn::Step::Draw
                && matches!(e.pending(), Pending::Priority { .. })
        });
        if !is_un {
            assert!(!stack_is_empty(&e));
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.players[1].life, life - i32::from(!is_un));
    }
}

#[test]
fn underdome_pays_an_already_triggered_optional_cost_after_the_un_source_leaves() {
    for is_un in [false, true] {
        let mut e = prepared(
            &[index::UNSUMMON],
            &[index::UNDERDOME, index::HASRAN_OGRESS],
            &[],
        );
        let ogress = on_battlefield(&e, P0, index::HASRAN_OGRESS).unwrap();
        print_fact(&mut e, ogress, is_un);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
        });
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogress, Defender::Player(P1))],
            },
        )
        .unwrap();
        priority(&mut e, P0);
        pool(&mut e, P0, &[(ManaColor::Blue, 1)]);
        let bounce = in_hand(&e, P0, index::UNSUMMON).unwrap();
        e.apply(P0, PlayerAction::CastSpell { card: bounce })
            .unwrap();
        targets(&mut e, vec![ogress], vec![]);
        pass_until(&mut e, |e| {
            e.state.object(ogress).unwrap().zone == Zone::Hand
                && matches!(e.pending(), Pending::Priority { player: P0, .. })
        });
        pool(&mut e, P0, &[]);
        restricted(&mut e, P0, ManaColor::Red);
        let land = on_battlefield(&e, P0, index::UNDERDOME).unwrap();
        e.state.set_tapped(land, false);
        e.refresh_offer();
        restricted(&mut e, P0, ManaColor::Red);
        let life = e.state.players[0].life;
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        e.apply(P0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.players[0].life, life - if is_un { 0 } else { 3 });
        assert_eq!(restricted_units(&e, P0), if is_un { 0 } else { 2 });
        assert_eq!(e.state.object(ogress).unwrap().zone, Zone::Hand);
    }
}
