//! Canonical Oracle behavior, exercised through real spells and combat.
// oracle_id = "d3088e1d-62c9-4478-9ef7-fc3c9e5cfadb"

#[allow(clippy::wildcard_imports)] // Shared canonical-card harness.
use super::*;
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
fn batch57_reverberation_redirects_target_sorcery_to_its_controller() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ISLAND; 4])
        .hand(0, &[index::REVERBERATION])
        .battlefield(1, &[index::MOUNTAIN; 2])
        .hand(1, &[index::VOLCANIC_HAMMER])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    cast_aim(&mut e, P1, index::VOLCANIC_HAMMER, &[], &[P0]);
    let hammer = on_stack(&e, index::VOLCANIC_HAMMER).unwrap();
    cast_aim(&mut e, P0, index::REVERBERATION, &[hammer], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 20);
    assert_eq!(e.state().players[1].life, 17);
    assert_eq!(e.state().per_turn.damage_dealt_to[1], 3);
    let log = e.state().per_turn.damage_log.last().unwrap();
    assert_eq!(
        log.source.object, hammer,
        "the sorcery remains the damage source"
    );
}

#[test]
fn batch57_reverberation_redirects_every_sweeper_recipient() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
            ],
        )
        .hand(0, &[index::REVERBERATION])
        .battlefield(1, &[index::MOUNTAIN, index::MOUNTAIN, index::GRIZZLY_BEARS])
        .hand(1, &[index::PYROCLASM])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    cast_from_hand(&mut e, P1, index::PYROCLASM);
    let pyro = on_stack(&e, index::PYROCLASM).unwrap();
    cast_aim(&mut e, P0, index::REVERBERATION, &[pyro], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[1].life,
        14,
        "three original recipients each redirect two"
    );
    for (seat, card) in [
        (P0, index::GRIZZLY_BEARS),
        (P0, index::LLANOWAR_ELVES),
        (P1, index::GRIZZLY_BEARS),
    ] {
        let id = on_battlefield(&e, seat, card).unwrap();
        assert_eq!(e.state().object(id).unwrap().damage, 0);
    }
}

#[test]
fn batch57_reverberation_cannot_target_an_instant() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ISLAND; 4])
        .hand(0, &[index::REVERBERATION])
        .battlefield(1, &[index::MOUNTAIN])
        .hand(1, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast_aim(&mut e, P1, index::LIGHTNING_BOLT, &[], &[P0]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    let reverb = in_hand(&e, P0, index::REVERBERATION).unwrap();
    assert!(
        !priority_offer(&e).castable.contains(&reverb),
        "all costs met but no sorcery target"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 17);
}
