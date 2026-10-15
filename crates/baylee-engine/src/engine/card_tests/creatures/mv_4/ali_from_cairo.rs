//! Canonical Oracle behavior, exercised through real spells and combat.
// oracle_id = "6517edb3-30e2-40ba-b6e4-4554b4bbb342"

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
fn batch57_ali_floors_damage_life_loss_without_reducing_damage() {
    for (start, expected) in [(2, 1), (6, 3)] {
        let mut e = Duel::new(SEED, index::FOREST)
            .life(0, start)
            .battlefield(0, &[index::ALI_FROM_CAIRO])
            .battlefield(1, &[index::MOUNTAIN])
            .hand(1, &[index::LIGHTNING_BOLT])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        cast_aim(&mut e, P1, index::LIGHTNING_BOLT, &[], &[P0]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, expected);
        assert_eq!(
            e.state().per_turn.damage_dealt_to[0],
            3,
            "full damage was dealt"
        );
        assert_eq!(e.state().players[1].life, 20);
    }
}

#[test]
fn batch57_ali_does_not_protect_against_non_damage_life_loss() {
    let mut e = Duel::new(SEED, index::FOREST)
        .life(0, 2)
        .battlefield(0, &[index::ALI_FROM_CAIRO])
        .battlefield(1, &[index::SWAMP; 5])
        .hand(1, &[index::SOUL_FEAST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    cast_from_hand(&mut e, P1, index::SOUL_FEAST);
    let Pending::ChoosePlayer { player, options } = e.pending() else {
        panic!("Soul Feast player target")
    };
    assert_eq!(*player, P1);
    assert!(options.contains(&P0));
    e.apply(P1, PlayerAction::ChoosePlayer(P0)).unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::GameOver(_)));
    assert_eq!(e.state().players[0].life, -2);
    assert_eq!(e.state().per_turn.damage_dealt_to[0], 0);
}

#[test]
fn batch57_ali_protects_during_simultaneous_lethal_damage_to_itself() {
    let mut e = Duel::new(SEED, index::FOREST)
        .life(0, 2)
        .battlefield(0, &[index::ALI_FROM_CAIRO])
        .battlefield(1, &[index::MOUNTAIN; 3])
        .hand(1, &[index::EARTHQUAKE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    cast_from_hand(&mut e, P1, index::EARTHQUAKE);
    let Pending::ChooseNumber { player, .. } = e.pending() else {
        panic!("Earthquake X")
    };
    assert_eq!(*player, P1);
    e.apply(P1, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 1);
    assert_eq!(e.state().players[1].life, 18);
    assert!(in_graveyard(&e, P0, index::ALI_FROM_CAIRO).is_some());
    assert_eq!(e.state().per_turn.damage_dealt_to[0], 2);
}

#[test]
fn batch57_ali_uses_simultaneous_lifelink_before_limiting_life_loss() {
    let mut e = Duel::new(SEED, index::FOREST)
        .life(0, 6)
        .battlefield(
            0,
            &[index::ALI_FROM_CAIRO, index::BRAD_BOIMLER_EAGER_ENSIGN],
        )
        .battlefield(1, &[index::CRAW_WURM, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let brad = on_battlefield(&e, P0, index::BRAD_BOIMLER_EAGER_ENSIGN).unwrap();
    let wurm = on_battlefield(&e, P1, index::CRAW_WURM).unwrap();
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    kit::attack_on_their_turn(&mut e, P1, &[wurm, bear], P0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(brad, bear)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        e.state().players[0].life,
        2,
        "six loss and two simultaneous lifelink, not floor then gain"
    );
    assert_eq!(e.state().per_turn.damage_dealt_to[0], 6);
}
