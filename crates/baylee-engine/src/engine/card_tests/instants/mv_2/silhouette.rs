//! Canonical Oracle behavior, exercised through real spells and combat.
// oracle_id = "5a074b0a-3a4d-4c6f-b161-5d1281f6136d"

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

#[test]
fn batch57_silhouette_prevents_targeted_damage_only_to_the_selected_creature_and_expires() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::ISLAND,
                index::ISLAND,
                index::ROOTBREAKER_WURM,
                index::CRAW_WURM,
            ],
        )
        .hand(0, &[index::SILHOUETTE])
        .battlefield(1, &[index::MOUNTAIN; 3])
        .hand(1, &[index::LIGHTNING_BOLT; 3])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let protected = on_battlefield(&e, P0, index::ROOTBREAKER_WURM).unwrap();
    let other = on_battlefield(&e, P0, index::CRAW_WURM).unwrap();
    cast_aim(&mut e, P0, index::SILHOUETTE, &[protected], &[]);
    pass_until(&mut e, stack_is_empty);
    cast_aim(&mut e, P1, index::LIGHTNING_BOLT, &[protected], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(protected).unwrap().damage, 0);
    cast_aim(&mut e, P1, index::LIGHTNING_BOLT, &[other], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(other).unwrap().damage, 3);
    reach_their_main_phase(&mut e, P1);
    cast_aim(&mut e, P1, index::LIGHTNING_BOLT, &[protected], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(protected).unwrap().damage, 3);
}

#[test]
fn batch57_silhouette_does_not_prevent_untargeted_damage() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ISLAND, index::ISLAND, index::ROOTBREAKER_WURM])
        .hand(0, &[index::SILHOUETTE])
        .battlefield(1, &[index::MOUNTAIN; 2])
        .hand(1, &[index::PYROCLASM])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    let protected = on_battlefield(&e, P0, index::ROOTBREAKER_WURM).unwrap();
    cast_from_hand(&mut e, P1, index::PYROCLASM);
    cast_aim(&mut e, P0, index::SILHOUETTE, &[protected], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(protected).unwrap().damage, 2);
}

#[test]
fn batch57_silhouette_can_protect_an_opponents_creature_from_targeted_abilities() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[index::ISLAND, index::ISLAND, index::PRODIGAL_PYROMANCER],
        )
        .hand(0, &[index::SILHOUETTE])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let pyro = on_battlefield(&e, P0, index::PRODIGAL_PYROMANCER).unwrap();
    cast_aim(&mut e, P0, index::SILHOUETTE, &[bear], &[]);
    pass_until(&mut e, stack_is_empty);
    kit::activate_object(&mut e, P0, pyro, 0);
    aim(&mut e, P0, &[bear], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(bear).unwrap().damage, 0);
    assert_eq!(e.state().players[1].life, 20);
}
