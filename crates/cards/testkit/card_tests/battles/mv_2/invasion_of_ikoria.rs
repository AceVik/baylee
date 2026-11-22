//! `cards/battles/mv_2/invasion_of_ikoria.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::{battle_damage_kit, legends_kit as kit};
use baylee_core::generated::index::{
    GRIZZLY_BEARS, LIGHTNING_BOLT, MOTHER_OF_RUNES, RUNECLAW_BEAR, SERRA_ANGEL,
};
use baylee_engine::choice::{ChoicePrompt, YesNoPrompt};
use baylee_engine::event::{Cause, GameEvent};
use baylee_engine::zone::ZonePosition;

fn invasion_of_ikoria() -> CardIndex {
    card_index("21d90735-5e04-4a8c-a546-ff26d4d5998d")
}

fn move_from_hand(e: &mut Engine<RegistryLookup>, card: CardIndex, to: ZoneLocation) -> ObjectId {
    let object = in_hand(e, kit::P0, card).unwrap();
    e.dev_state_mut(kit::P0)
        .unwrap()
        .move_object(object, to, ZonePosition::Top, Cause::Effect)
        .unwrap()
}

fn cast_with_x(e: &mut Engine<RegistryLookup>, x: u32) -> ObjectId {
    kit::float(e, kit::P0, &[(ManaColor::Green, x + 2)]);
    cast_with_floating(e, kit::P0, invasion_of_ikoria());
    assert!(matches!(e.pending(), Pending::ChooseNumber { player, .. } if *player == kit::P0));
    e.apply(kit::P0, PlayerAction::ChooseNumber(x)).unwrap();
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::SearchLibrary,
                ..
            }
        )
    });
    assert_eq!(e.pending().asked(), Some(kit::P0));
    let siege = on_battlefield(e, kit::P0, invasion_of_ikoria()).unwrap();
    assert_eq!(battle_damage_kit::defense(e, siege), 6);
    assert_eq!(battle_damage_kit::protector(e, siege), kit::P1);
    siege
}

fn shuffles(e: &Engine<RegistryLookup>) -> usize {
    e.state()
        .journal
        .entries()
        .iter()
        .filter(|entry| {
            matches!(
                entry.event,
                GameEvent::Shuffled {
                    player: kit::P0,
                    zone: Zone::Library
                }
            )
        })
        .count()
}

#[test]
fn ikoria_uses_cast_x_to_find_a_non_human_in_either_zone_and_shuffles_only_if_searched() {
    for (search_library, find) in [
        (false, None),
        (false, Some(Zone::Graveyard)),
        (true, Some(Zone::Graveyard)),
        (true, Some(Zone::Library)),
    ] {
        let mut e = Duel::new(8303, forest())
            .hand(
                0,
                &[
                    invasion_of_ikoria(),
                    GRIZZLY_BEARS,
                    RUNECLAW_BEAR,
                    MOTHER_OF_RUNES,
                    SERRA_ANGEL,
                ],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, kit::P0);
        let graveyard = ZoneLocation::Graveyard(kit::P0);
        let library = ZoneLocation::Library(kit::P0);
        let bear = find
            .is_some()
            .then(|| move_from_hand(&mut e, GRIZZLY_BEARS, graveyard));
        let library_bear = move_from_hand(&mut e, RUNECLAW_BEAR, library);
        move_from_hand(&mut e, MOTHER_OF_RUNES, graveyard);
        move_from_hand(
            &mut e,
            SERRA_ANGEL,
            if search_library { library } else { graveyard },
        );
        let before = shuffles(&e);

        cast_with_x(&mut e, 2);
        e.apply(kit::P0, PlayerAction::YesNo(search_library))
            .unwrap();
        if let Some(bear) = bear {
            if search_library {
                assert!(matches!(e.pending(), Pending::ChooseCards {
                    player, options, min: 0, max: 1, prompt: ChoicePrompt::SearchLibraryOrGraveyard, ..
                } if *player == kit::P0 && options.len() == 2
                    && options.contains(&bear) && options.contains(&library_bear)));
            } else {
                assert!(matches!(e.pending(), Pending::ChooseCards {
                    player, options, min: 1, max: 1, prompt: ChoicePrompt::FromGraveyard, ..
                } if *player == kit::P0 && options.as_slice() == [bear]));
            }
            let found = if find == Some(Zone::Library) {
                library_bear
            } else {
                bear
            };
            e.apply(
                kit::P0,
                PlayerAction::ChooseObjects {
                    objects: vec![found],
                },
            )
            .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            on_battlefield(&e, kit::P0, GRIZZLY_BEARS),
            bear.filter(|_| find == Some(Zone::Graveyard))
        );
        assert_eq!(
            on_battlefield(&e, kit::P0, RUNECLAW_BEAR),
            (find == Some(Zone::Library)).then_some(library_bear)
        );
        assert_eq!(shuffles(&e), before + usize::from(search_library));
    }
}

fn enter_zilortha(e: &mut Engine<RegistryLookup>) -> ObjectId {
    let p0 = kit::P0;
    let siege = cast_with_x(e, 0);
    e.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(e, stack_is_empty);
    for _ in 0..2 {
        kit::float(e, p0, &[(ManaColor::Red, 1)]);
        cast_with_floating(e, p0, LIGHTNING_BOLT);
        kit::aim(e, &[siege], &[]);
        pass_until(e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::PlayFromExile,
                    ..
                }
            ) || stack_is_empty(e)
        });
    }
    battle_damage_kit::await_defeat_cast(e, siege);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![siege],
        },
    )
    .unwrap();
    pass_until(e, stack_is_empty);
    siege
}

fn attack_and_block(
    e: &mut Engine<RegistryLookup>,
    attacker: PlayerId,
    defender: PlayerId,
    attackers: &[ObjectId],
    blockers: Vec<(ObjectId, ObjectId)>,
) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == attacker),
    );
    e.apply(
        attacker,
        PlayerAction::DeclareAttackers {
            attackers: attackers
                .iter()
                .map(|&id| (id, Defender::Player(defender)))
                .collect(),
        },
    )
    .unwrap();
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == defender),
    );
    e.apply(defender, PlayerAction::DeclareBlockers { blockers })
        .unwrap();
}

#[test]
fn zilortha_is_green_and_only_lets_non_humans_you_control_assign_unblocked_damage() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut e = Duel::new(8304, forest())
        .battlefield(0, &[GRIZZLY_BEARS, MOTHER_OF_RUNES])
        .battlefield(1, &[RUNECLAW_BEAR, SERRA_ANGEL])
        .hand(0, &[invasion_of_ikoria(), LIGHTNING_BOLT, LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let siege = enter_zilortha(&mut e);
    let back = e.state().object(siege).unwrap();
    assert_eq!(back.face_index, 1);
    assert_eq!(
        back.characteristics().colors,
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Green),
        "CR 202.2e gives Zilortha its printed green color indicator",
    );
    assert_eq!(pt(&e, siege), (8, 8));
    assert!(keywords(&e, siege).contains(KeywordSet::REACH));

    let bear = on_battlefield(&e, p0, GRIZZLY_BEARS).unwrap();
    let human = on_battlefield(&e, p0, MOTHER_OF_RUNES).unwrap();
    let defending_bear = on_battlefield(&e, p1, RUNECLAW_BEAR).unwrap();
    let angel = on_battlefield(&e, p1, SERRA_ANGEL).unwrap();
    attack_and_block(
        &mut e,
        p0,
        p1,
        &[bear, human],
        vec![(defending_bear, bear), (angel, human)],
    );
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert!(matches!(e.pending(), Pending::YesNo {
        player, prompt: YesNoPrompt::AssignDamageAsThoughUnblocked { attacker }, ..
    } if *player == p0 && *attacker == bear));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, |e| kit::life(e, p1) == 18);
    assert_eq!(e.state().object(defending_bear).unwrap().damage, 0);
    assert_eq!(
        e.state().object(angel).unwrap().damage,
        1,
        "the Human deals damage to its blocker"
    );

    attack_and_block(
        &mut e,
        p1,
        p0,
        &[defending_bear],
        vec![(siege, defending_bear)],
    );
    pass_until(&mut e, |e| {
        e.state().object(defending_bear).unwrap().zone == Zone::Graveyard
    });
    assert_eq!(kit::life(&e, p0), 20);
    assert_eq!(e.state().object(siege).unwrap().damage, 2);
}
