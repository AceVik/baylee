//! Inherited activated definitions keep their printed zones and conditions.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch46_agatha_preserves_graveyard_activation_zone_and_ferocious_condition() {
    let mut e = prepared(
        &[index::FANATIC_OF_RHONAS],
        &[index::AGATHA_S_SOUL_CAULDRON, index::GRIZZLY_BEARS],
        &[],
    );
    let fanatic = hand_to_graveyard(&mut e, P0, index::FANATIC_OF_RHONAS);
    let bearer = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    exile(&mut e, fanatic, Some(bearer));
    let ids = grants(&e, bearer);
    assert_eq!(ids.len(), 3);
    reset_pool(&mut e);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 4);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(legal.abilities.contains(&(bearer, ids[0])));
    assert!(
        !legal.abilities.contains(&(bearer, ids[1])),
        "bearer has power three, so ferocious is false"
    );
    assert!(
        !legal.abilities.contains(&(bearer, ids[2])),
        "eternalize is only activated from a graveyard"
    );
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: bearer,
                ability_index: ids[2]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    e.state
        .object_mut(bearer)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 1);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        legal.abilities.contains(&(bearer, ids[1])),
        "the exact inherited condition becomes true at power four"
    );
    assert!(!legal.abilities.contains(&(bearer, ids[2])));
    reset_pool(&mut e);
    use_grant(&mut e, bearer, ids[1]);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Green), 4);
    assert_eq!(e.state.players[0].mana_pool.total(), 4);
    assert!(is_tapped(&e, bearer));
    assert!(stack_is_empty(&e));
}
