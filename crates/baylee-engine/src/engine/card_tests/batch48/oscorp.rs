//! Actual card actions and historical identity boundaries.
#[allow(clippy::wildcard_imports)]
use super::*;

fn discard_oscorp(e: &mut Engine<RegistryLookup>) -> ObjectId {
    let land = in_hand(e, P0, index::OSCORP_INDUSTRIES).unwrap();
    ability(e, P0, index::TIRELESS_TRIBE, 0);
    choose(e, vec![land]);
    pass_until(e, stack_is_empty);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Graveyard);
    assert!(
        e.state
            .object(land)
            .unwrap()
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::MAYHEM),
        "Mayhem is a native keyword on the graveyard card"
    );
    land
}

fn reject_play(e: &mut Engine<RegistryLookup>, land: ObjectId) {
    priority(e, P0);
    e.refresh_offer();
    let before = e.snapshot_hash();
    assert!(e.apply(P0, PlayerAction::PlayLand { card: land }).is_err());
    assert_eq!(e.snapshot_hash(), before);
}

#[test]
fn batch48_oscorp_discarded_land_uses_normal_land_allowance_and_enters_tapped() {
    for used_land in [false, true] {
        let mut e = prepared(
            &[index::OSCORP_INDUSTRIES, index::FOREST],
            &[index::TIRELESS_TRIBE],
            &[],
        );
        if used_land {
            let forest = in_hand(&e, P0, index::FOREST).unwrap();
            e.apply(P0, PlayerAction::PlayLand { card: forest })
                .unwrap();
            pass_until(&mut e, stack_is_empty);
        }
        let land = discard_oscorp(&mut e);
        if used_land {
            reject_play(&mut e, land);
        } else {
            let life = e.state.players[0].life;
            priority(&mut e, P0);
            e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
            assert!(is_tapped(&e, land));
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state.players[0].life, life - 2);
        }
    }
}

#[test]
fn batch48_oscorp_hand_entry_has_no_life_loss_and_mana_is_ubr() {
    let mut e = prepared(&[index::OSCORP_INDUSTRIES], &[], &[]);
    let land = in_hand(&e, P0, index::OSCORP_INDUSTRIES).unwrap();
    let life = e.state.players[0].life;
    e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, land));
    assert_eq!(e.state.players[0].life, life);
    e.state.set_tapped(land, false);
    e.refresh_offer();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: 1,
        },
    )
    .unwrap();
    let Pending::ChooseColor { options, .. } = e.pending() else {
        panic!("mana choice")
    };
    assert_eq!(options.len(), 3);
    for color in [ManaColor::Blue, ManaColor::Black, ManaColor::Red] {
        assert!(options.contains(&color));
    }
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
}

#[test]
fn batch48_oscorp_mayhem_requires_own_turn_and_expires_at_next_turn() {
    let mut e = prepared(&[index::OSCORP_INDUSTRIES], &[index::TIRELESS_TRIBE], &[]);
    let land = discard_oscorp(&mut e);
    next_main(&mut e, P1);
    reject_play(&mut e, land);
    next_main(&mut e, P0);
    reject_play(&mut e, land);
}

#[test]
fn batch48_oscorp_mayhem_does_not_survive_a_second_graveyard_incarnation() {
    let mut e = prepared(&[index::OSCORP_INDUSTRIES], &[index::TIRELESS_TRIBE], &[]);
    let land = discard_oscorp(&mut e);
    // Isolate the intervening zone changes, which must break the permission.
    e.state
        .move_object(
            land,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            land,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    reject_play(&mut e, land);
}

#[test]
fn batch48_oscorp_non_discard_move_needs_independent_excavator_permission() {
    for excavator in [false, true] {
        let board = if excavator {
            vec![index::RAMUNAP_EXCAVATOR]
        } else {
            vec![]
        };
        let mut e = prepared(&[index::OSCORP_INDUSTRIES], &board, &[]);
        let land = in_hand(&e, P0, index::OSCORP_INDUSTRIES).unwrap();
        // A non-discard zone move isolates absence of the discard event.
        e.state
            .move_object(
                land,
                ZoneLocation::Graveyard(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        if excavator {
            let life = e.state.players[0].life;
            e.refresh_offer();
            e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state.players[0].life, life - 2);
        } else {
            reject_play(&mut e, land);
        }
    }
}

#[test]
fn batch48_oscorp_entry_from_an_opponents_graveyard_charges_the_entry_controller() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(1, &[index::OSCORP_INDUSTRIES])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let land = in_hand(&e, P1, index::OSCORP_INDUSTRIES).unwrap();
    e.state
        .move_object(
            land,
            ZoneLocation::Graveyard(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    // Model an effect putting an opponent-owned card onto the battlefield
    // under P0's control. The card remains in its OWNER's graveyard until entry.
    let object = e.state.object_mut(land).unwrap();
    object.controller = P0;
    object.base_controller = P0;
    let before = [e.state.players[0].life, e.state.players[1].life];
    e.state
        .move_object(
            land,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, land));
    assert_eq!(
        [e.state.players[0].life, e.state.players[1].life],
        [before[0] - 2, before[1]]
    );
}

fn next_main(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == player
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player)
    });
}

#[test]
fn batch48_oscorp_fresh_discard_grants_new_incarnation_permission() {
    let mut e = prepared(&[index::OSCORP_INDUSTRIES], &[index::TIRELESS_TRIBE], &[]);
    let land = discard_oscorp(&mut e);
    let discarded = e.state.object(land).unwrap().version;
    // Transparent intervening retrieval. The second discard is a real cost.
    e.state
        .move_object(
            land,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let land = discard_oscorp(&mut e);
    assert_ne!(e.state.object(land).unwrap().version, discarded);
    priority(&mut e, P0);
    let life = e.state.players[0].life;
    e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life - 2);
}

#[test]
fn batch48_oscorp_mayhem_does_not_bypass_empty_stack_requirement() {
    let mut e = prepared(
        &[index::OSCORP_INDUSTRIES, index::DARK_RITUAL],
        &[index::TIRELESS_TRIBE],
        &[],
    );
    let land = discard_oscorp(&mut e);
    cast(&mut e, index::DARK_RITUAL);
    assert!(!stack_is_empty(&e));
    reject_play(&mut e, land);
    pass_until(&mut e, stack_is_empty);
    priority(&mut e, P0);
    e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, land));
}

#[test]
fn batch48_oscorp_leng_does_not_replace_an_activation_discard_cost() {
    let mut e = prepared(
        &[index::OSCORP_INDUSTRIES],
        &[index::TIRELESS_TRIBE, index::LIBRARY_OF_LENG],
        &[],
    );
    let land = discard_oscorp(&mut e);
    priority(&mut e, P0);
    e.apply(P0, PlayerAction::PlayLand { card: land }).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, land));
    assert_eq!(e.state.players[0].life, 18);
}

#[test]
fn batch48_oscorp_leng_replaced_effect_discard_does_not_grant_graveyard_play() {
    let mut e = prepared(
        &[index::MOX_DIAMOND, index::OSCORP_INDUSTRIES],
        &[index::LIBRARY_OF_LENG],
        &[],
    );
    let land = in_hand(&e, P0, index::OSCORP_INDUSTRIES).unwrap();
    cast(&mut e, index::MOX_DIAMOND);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    choose(&mut e, vec![land]);
    assert!(matches!(
        e.pending(),
        Pending::Arrange {
            prompt: crate::choice::ArrangePrompt::DiscardToLibrary,
            ..
        }
    ));
    e.apply(
        P0,
        PlayerAction::Arrange {
            piles: vec![vec![], vec![land]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Library);
    // Isolated subsequent move is not a fresh discard: Leng's replacement
    // must not leave a Mayhem entitlement for this later graveyard incarnation.
    e.state
        .move_object(
            land,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    reject_play(&mut e, land);
}

#[test]
fn batch48_oscorp_cleanup_discard_does_not_carry_permission_into_another_turn() {
    let mut hand = vec![index::FOREST; 7];
    hand.push(index::OSCORP_INDUSTRIES);
    let mut e = prepared(&hand, &[], &[]);
    let land = in_hand(&e, P0, index::OSCORP_INDUSTRIES).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::DiscardChoice { player: P0, .. })
    });
    let Pending::DiscardChoice { count, .. } = e.pending() else {
        unreachable!()
    };
    let mut discarded = vec![land];
    discarded.extend(
        e.state
            .zones
            .list(ZoneLocation::Hand(P0))
            .iter()
            .copied()
            .filter(|id| *id != land)
            .take(usize::from(*count) - 1),
    );
    e.apply(P0, PlayerAction::ChooseObjects { objects: discarded })
        .unwrap();
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Graveyard);
    next_main(&mut e, P0);
    reject_play(&mut e, land);
}
