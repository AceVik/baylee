//! Canonical Oracle behavior, exercised through real spells and combat.
// oracle_id = "db121503-8a34-498a-829c-72c33798369b"

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

fn nova_game(hand1: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::NOVA_PENTACLE,
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::ROOTBREAKER_WURM,
            ],
        )
        .battlefield(
            1,
            &[
                index::PRODIGAL_PYROMANCER,
                index::GRIZZLY_BEARS,
                index::ISLAND,
            ],
        )
        .hand(1, hand1)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    e
}

fn establish_nova(e: &mut Engine<RegistryLookup>) -> (ObjectId, ObjectId, ObjectId) {
    let nova = on_battlefield(e, P0, index::NOVA_PENTACLE).unwrap();
    let wurm = on_battlefield(e, P0, index::ROOTBREAKER_WURM).unwrap();
    let pyro = on_battlefield(e, P1, index::PRODIGAL_PYROMANCER).unwrap();
    let bear = on_battlefield(e, P1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(e, P0);
    kit::activate_object(e, P0, nova, 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = e.pending()
    else {
        panic!("opponent chooses creature: {:?}", e.pending())
    };
    assert_eq!(*player, P1);
    assert_eq!((*min, *max), (1, 1));
    assert!(options.contains(&wurm) && options.contains(&bear));
    assert!(!options.contains(&nova));
    assert!(!is_tapped(e, nova));
    assert_eq!(e.state().players[0].mana_pool.total(), 3);
    aim(e, P1, &[wurm], &[]);
    assert!(is_tapped(e, nova));
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    choose_source(e, pyro);
    (nova, wurm, pyro)
}

#[test]
fn batch57_nova_opponent_selects_target_before_costs_and_redirects_only_next_source_event() {
    let mut e = nova_game(&[index::TWIDDLE]);
    let (_, wurm, pyro) = establish_nova(&mut e);
    kit::activate_object(&mut e, P1, pyro, 0);
    aim(&mut e, P1, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 20);
    assert_eq!(e.state().object(wurm).unwrap().damage, 1);
    assert_eq!(e.state().players[1].life, 20);
    cast_aim(&mut e, P1, index::TWIDDLE, &[pyro], &[]);
    pass_until(&mut e, stack_is_empty);
    kit::activate_object(&mut e, P1, pyro, 0);
    aim(&mut e, P1, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(e.state().object(wurm).unwrap().damage, 1);
}

#[test]
fn batch57_nova_departed_destination_does_not_redirect_and_unused_shield_expires() {
    for remove in [false, true] {
        let mut e = nova_game(&[index::UNSUMMON]);
        let (_, wurm, pyro) = establish_nova(&mut e);
        if remove {
            cast_aim(&mut e, P1, index::UNSUMMON, &[wurm], &[]);
            pass_until(&mut e, stack_is_empty);
            assert!(in_hand(&e, P0, index::ROOTBREAKER_WURM).is_some());
        } else {
            reach_their_main_phase(&mut e, P0);
            reach_their_main_phase(&mut e, P1);
        }
        kit::activate_object(&mut e, P1, pyro, 0);
        aim(&mut e, P1, &[], &[P0]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, 19);
        assert_eq!(e.state().players[1].life, 20);
    }
}

#[test]
fn batch57_nova_requires_three_mana_and_a_legal_creature_target() {
    for (lands, creature) in [(2, true), (3, false)] {
        let mut board = vec![index::NOVA_PENTACLE];
        board.extend(vec![index::FOREST; lands]);
        let others = if creature {
            vec![index::GRIZZLY_BEARS]
        } else {
            vec![]
        };
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(0, &board)
            .battlefield(1, &others)
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        let nova = on_battlefield(&e, P0, index::NOVA_PENTACLE).unwrap();
        tap_all_mana(&mut e, P0);
        assert!(!priority_offer(&e).abilities.contains(&(nova, 0)));
        assert!(!is_tapped(&e, nova));
    }
}
