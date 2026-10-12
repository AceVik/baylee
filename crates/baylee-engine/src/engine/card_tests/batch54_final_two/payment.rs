#[allow(clippy::wildcard_imports)]
use super::*;

fn enter_forest(e: &mut Engine<RegistryLookup>) -> ObjectId {
    let portal = play_land(e, P0, index::THRAN_PORTAL);
    assert!(matches!(e.pending(),Pending::ChooseSubtype{options,..} if options.len()==5));
    e.apply(P0, PlayerAction::ChooseSubtype(subtypes::land::FOREST))
        .unwrap();
    assert_eq!(e.state.object(portal).unwrap().zone, Zone::Battlefield);
    portal
}

#[test]
fn thran_two_other_lands_untapped_three_other_lands_tapped() {
    for count in [2, 3] {
        let mine = vec![index::FOREST; count];
        let mut e = prepared(&[index::THRAN_PORTAL], &mine, &[]);
        let portal = enter_forest(&mut e);
        assert_eq!(is_tapped(&e, portal), count == 3);
    }
}

#[test]
fn thran_insufficient_life_rejects_actual_activation_atomically() {
    let mut e = prepared(&[index::THRAN_PORTAL], &[], &[]);
    let portal = enter_forest(&mut e);
    reset(&mut e);
    // Boundary fixture: no automatic SBA is invoked when setting the account.
    // This tests cost refusal itself, not a purported playable zero-life board.
    e.state.players[0].life = 0;
    e.refresh_offer();
    let before = e.snapshot_hash();
    assert!(
        e.apply(P0, PlayerAction::ActivateManaAbility { source: portal })
            .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    assert!(!is_tapped(&e, portal));
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
}

#[test]
fn thran_entering_after_teferi_cannot_pay_life_for_intrinsic_or_granted_mana() {
    let mut e = prepared(
        &[
            index::TEFERI_S_PROTECTION,
            index::THRAN_PORTAL,
            index::CHROMATIC_LANTERN,
        ],
        &[],
        &[],
    );
    let spell = in_hand(&e, P0, index::TEFERI_S_PROTECTION).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    let lantern = in_hand(&e, P0, index::CHROMATIC_LANTERN).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: lantern })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    let portal = enter_forest(&mut e);
    reset(&mut e);
    let grant = crate::effects::granted_activated(e.state(), portal)
        .next()
        .unwrap()
        .id;
    // Both intrinsic and granted mana must disappear from the payable offer.
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.abilities.iter().any(|(id, _)| *id == portal));
    assert!(!legal.mana_abilities.contains(&portal));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: portal,
                ability_index: grant
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    assert!(
        e.apply(P0, PlayerAction::ActivateManaAbility { source: portal })
            .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    assert!(!is_tapped(&e, portal));
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
}

#[test]
fn thran_positive_life_cost_is_paid_exactly_once_before_output() {
    let mut e = prepared(&[index::THRAN_PORTAL], &[], &[]);
    let portal = enter_forest(&mut e);
    reset(&mut e);
    e.state.players[0].life = 2;
    e.refresh_offer();
    tap_source(&mut e, P0, portal);
    assert_eq!(e.state.players[0].life, 1);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&e, portal));
    assert!(stack_is_empty(&e));
}

#[test]
fn cancelling_actual_cast_restores_thran_life_tap_and_mana() {
    let mut e = prepared(
        &[index::THRAN_PORTAL, index::GRIZZLY_BEARS],
        &[index::FOREST],
        &[],
    );
    let portal = enter_forest(&mut e);
    reset(&mut e);
    let card = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let life = e.state.players[0].life;
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    assert_eq!(e.cancellable_cast(P0), Some(card));
    e.apply(P0, PlayerAction::ActivateManaAbility { source: portal })
        .unwrap();
    assert_eq!(e.state.players[0].life, life - 1);
    assert!(is_tapped(&e, portal));
    e.apply(P0, PlayerAction::CancelCast).unwrap();
    assert_eq!(e.state.object(card).unwrap().zone, Zone::Hand);
    assert_eq!(e.state.players[0].life, life);
    assert!(!is_tapped(&e, portal));
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    assert!(stack_is_empty(&e));
}
