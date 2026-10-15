//! Canonical Oracle behavior, exercised through real spells and combat.
// oracle_id = "22647b1a-5a7c-41b5-b820-b2e9f49c7aad"

#[allow(clippy::wildcard_imports)] // Shared canonical-card harness.
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aim(e: &mut Engine<RegistryLookup>, seat: PlayerId, objects: &[ObjectId], players: &[PlayerId]) {
    let Pending::ChooseTargets { player, .. } = e.pending() else {
        panic!("target question: {:?}", e.pending())
    };
    assert_eq!(*player, seat);
    e.apply(
        seat,
        PlayerAction::ChooseTargets {
            objects: objects.to_vec(),
            players: players.to_vec(),
        },
    )
    .unwrap();
}

fn cast_aim(
    e: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    cast_from_hand(e, seat, card);
    aim(e, seat, objects, players);
}

fn choose_source(e: &mut Engine<RegistryLookup>, source: ObjectId) {
    pass_until(e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        choice,
        options,
        ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, P0);
    let source = options
        .into_iter()
        .find(|r| r.object == source)
        .expect("chosen source offered");
    e.apply(P0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(e, stack_is_empty);
}

#[test]
fn batch57_eye_reflects_only_the_next_chosen_source_event_and_preserves_sources() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::PLAINS; 2])
        .hand(0, &[index::EYE_FOR_AN_EYE])
        .battlefield(
            1,
            &[index::PRODIGAL_PYROMANCER, index::ISLAND, index::MOUNTAIN],
        )
        .hand(1, &[index::TWIDDLE, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    let pyro = on_battlefield(&e, P1, index::PRODIGAL_PYROMANCER).unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    cast_from_hand(&mut e, P0, index::EYE_FOR_AN_EYE);
    choose_source(&mut e, pyro);
    cast_aim(&mut e, P1, index::LIGHTNING_BOLT, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        17,
        "unchosen source deals its full damage"
    );
    assert_eq!(
        e.state().players[1].life,
        20,
        "unchosen source neither reflects nor consumes the shield"
    );
    kit::activate_object(&mut e, P1, pyro, 0);
    aim(&mut e, P1, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 16);
    assert_eq!(e.state().players[1].life, 19);
    let log = &e.state().per_turn.damage_log;
    assert!(log.iter().any(|d| d.source.object == pyro
        && d.to == crate::event::DamageTarget::Player(P0)
        && d.amount == 1));
    assert!(log.iter().any(|d| {
        d.snapshot
            .card
            .is_some_and(|c| c.index == index::EYE_FOR_AN_EYE)
            && d.to == crate::event::DamageTarget::Player(P1)
            && d.amount == 1
    }));
    cast_aim(&mut e, P1, index::TWIDDLE, &[pyro], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, pyro));
    kit::activate_object(&mut e, P1, pyro, 0);
    aim(&mut e, P1, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 15);
    assert_eq!(e.state().players[1].life, 19);
}

#[test]
fn batch57_eye_unused_choice_expires_at_end_of_turn() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::PLAINS; 2])
        .hand(0, &[index::EYE_FOR_AN_EYE])
        .battlefield(1, &[index::PRODIGAL_PYROMANCER])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let pyro = on_battlefield(&e, P1, index::PRODIGAL_PYROMANCER).unwrap();
    cast_from_hand(&mut e, P0, index::EYE_FOR_AN_EYE);
    choose_source(&mut e, pyro);
    reach_their_main_phase(&mut e, P1);
    kit::activate_object(&mut e, P1, pyro, 0);
    aim(&mut e, P1, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(e.state().players[1].life, 20);
}

#[test]
fn batch57_eye_reflects_to_the_chosen_sources_controller_even_when_that_is_you() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::PLAINS, index::PLAINS, index::ORCISH_ARTILLERY])
        .hand(0, &[index::EYE_FOR_AN_EYE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let artillery = on_battlefield(&e, P0, index::ORCISH_ARTILLERY).unwrap();
    cast_from_hand(&mut e, P0, index::EYE_FOR_AN_EYE);
    choose_source(&mut e, artillery);
    kit::activate_object(&mut e, P0, artillery, 0);
    aim(&mut e, P0, &[], &[P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        14,
        "three original self-damage plus three Eye damage"
    );
    assert_eq!(e.state().players[1].life, 18);
}
