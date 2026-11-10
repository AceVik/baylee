//! Additional root-requested activation-observer acceptance regressions.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn refused_vault_payment_reverses_earlier_tap_trigger_and_scan_cursor() {
    let mut e = prepared(
        &[],
        &[
            index::HAUNTING_WIND,
            index::PHYREXIAN_VAULT,
            index::LLANOWAR_ELVES,
            index::LLANOWAR_ELVES,
        ],
        &[],
    );
    let vault = source(&e, P0, index::PHYREXIAN_VAULT);
    let life = e.state.players[0].life;
    let mana = e.state.players[0].mana_pool.total();
    ability(&mut e, P0, index::PHYREXIAN_VAULT, 0);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("sacrifice cost");
    };
    let gone = options[0];
    // Same deliberate stale offered-choice fixture as engine reversal_tests:
    // no printed card removes a creature in the middle of a cost payment.
    assert!(e.state.zones.remove(gone, ZoneLocation::Battlefield));
    assert!(e.state.arena.remove(gone).is_some());
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![gone],
        },
    )
    .unwrap();
    assert!(matches!(e.pending(), Pending::Priority { player, .. } if *player == P0));
    assert!(stack_is_empty(&e));
    assert!(!is_tapped(&e, vault));
    assert_eq!(e.state.players[0].mana_pool.total(), mana);
    assert_eq!(e.state.players[0].life, life);
    assert!(
        e.trigger_queue.is_empty(),
        "no rolled-back tap trigger survives"
    );
    assert!(
        e.trigger_scan_seq <= e.state.journal.last_seq(),
        "cursor does not point beyond restored journal"
    );
    // A subsequent real successful activation proves neither a ghost trigger
    // nor a cursor which skips new events survived reversal.
    ability(&mut e, P0, index::PHYREXIAN_VAULT, 0);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("remaining sacrifice");
    };
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![options[0]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life - 1);
}

#[test]
fn observer_damage_follows_controller_change_after_activation_before_resolution() {
    for aura in [false, true] {
        let observer = if aura {
            index::ARTIFACT_POSSESSION
        } else {
            index::HAUNTING_WIND
        };
        let mut hand = vec![index::STEAL_ARTIFACT];
        if aura {
            hand.push(observer);
        }
        let board = if aura {
            vec![index::EMERGENCE_ZONE]
        } else {
            vec![index::EMERGENCE_ZONE, observer]
        };
        let mut e = prepared(&hand, &board, &[index::BASALT_MONOLITH]);
        let monolith = source(&e, P1, index::BASALT_MONOLITH);
        if aura {
            cast(&mut e, observer);
            targets(&mut e, vec![monolith], vec![]);
            pass_until(&mut e, stack_is_empty);
        }
        ability(&mut e, P0, index::EMERGENCE_ZONE, 1);
        pass_until(&mut e, stack_is_empty);
        for color in ManaColor::ALL {
            e.state.players[1].mana_pool.add(color, 30);
        }
        e.refresh_offer();
        let before = [e.state.players[0].life, e.state.players[1].life];
        ability(&mut e, P1, index::BASALT_MONOLITH, 2);
        // The tapless activation observer is now above the untap ability.
        // Emergence Zone permits the real Aura spell during this response.
        cast(&mut e, index::STEAL_ARTIFACT);
        targets(&mut e, vec![monolith], vec![]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.object(monolith).unwrap().controller, P0);
        assert_eq!(
            e.state.players[0].life,
            before[0] - if aura { 2 } else { 1 }
        );
        assert_eq!(e.state.players[1].life, before[1]);
    }
}
