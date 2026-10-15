//! Canonical Oracle behavior, exercised through real spells and combat.
// oracle_id = "0edf0988-9ed8-4fe6-aa47-4921870f05a8"

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
fn batch57_forethought_caps_instant_and_sorcery_damage_only_above_two() {
    for (spell, expected) in [
        (index::LIGHTNING_BOLT, 2),
        (index::SHOCK, 2),
        (index::VOLCANIC_HAMMER, 2),
        (index::SCORCHING_SPEAR, 1),
        (index::ZAP, 1),
    ] {
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(0, &[index::MOUNTAIN; 3])
            .hand(0, &[spell])
            .battlefield(1, &[index::FORETHOUGHT_AMULET])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        cast_aim(&mut e, P0, spell, &[], &[P1]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[1].life, 20 - expected);
        assert_eq!(
            e.state().per_turn.damage_dealt_to[1],
            u32::try_from(expected).unwrap()
        );
        assert_eq!(e.state().players[0].life, 20);
    }
}

#[test]
fn batch57_forethought_does_not_cap_creature_sources_or_other_players() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::MOUNTAIN,
                index::ORCISH_ARTILLERY,
            ],
        )
        .hand(0, &[index::FORETHOUGHT_AMULET, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let artillery = on_battlefield(&e, P0, index::ORCISH_ARTILLERY).unwrap();
    let mountain = on_battlefield(&e, P0, index::MOUNTAIN).unwrap();
    tap_mana_where(&mut e, P0, |id| id != artillery && id != mountain);
    cast_with_floating(&mut e, P0, index::FORETHOUGHT_AMULET);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::FORETHOUGHT_AMULET).is_some());
    kit::activate_object(&mut e, P0, artillery, 0);
    aim(&mut e, P0, &[], &[P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        17,
        "creature-source damage is not capped"
    );
    assert_eq!(e.state().players[1].life, 18);
    cast_aim(&mut e, P0, index::LIGHTNING_BOLT, &[], &[P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[1].life,
        15,
        "amulet protects only its controller"
    );
}

#[test]
fn batch57_forethought_upkeep_requires_exactly_three_or_sacrifice() {
    for pay in [false, true] {
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(
                1,
                &[
                    index::FORETHOUGHT_AMULET,
                    index::FOREST,
                    index::FOREST,
                    index::FOREST,
                ],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        pass_until(&mut e, |e| {
            e.state().turn.active == P1
                && e.state().turn.step == Step::Upkeep
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
        });
        if pay {
            assert_eq!(tap_all_mana(&mut e, P1), 3);
        }
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        let Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::PayTax { mana },
            ..
        } = e.pending()
        else {
            panic!("upkeep payment")
        };
        assert_eq!((*player, *mana), (P1, 3));
        e.apply(P1, PlayerAction::YesNo(pay)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            on_battlefield(&e, P1, index::FORETHOUGHT_AMULET).is_some(),
            pay
        );
        assert_eq!(
            in_graveyard(&e, P1, index::FORETHOUGHT_AMULET).is_some(),
            !pay
        );
        assert_eq!(e.state().players[1].mana_pool.total(), 0);
    }
}
