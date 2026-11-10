//! `cards/sorceries/mv_2/malevolent_rumble.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::{legends_kit as lk, partials_w1b_kit as kit};
use baylee_core::generated::{index, subtypes};

const P0: PlayerId = PlayerId::new(0);

fn malevolent_rumble() -> CardIndex {
    card_index("d30d8968-ec24-4155-97e2-28efd52c8810")
}

struct Cast {
    engine: Engine<RegistryLookup>,
    stacked: Vec<ObjectId>,
}

/// Casts Rumble for its printed cost with exactly `top_first` in the library.
fn cast(top_first: &[CardIndex]) -> Cast {
    let rumble = malevolent_rumble();
    let mut hand = vec![rumble];
    hand.extend_from_slice(top_first);
    let mut engine = lk::game(SEED, &[], &[], &hand, &[]);
    let stacked = kit::replace_library(&mut engine, P0, top_first);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
    );
    cast_with_floating(&mut engine, P0, rumble);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Green), 0);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Colorless), 0);
    assert_eq!(lk::pool_total(&engine, P0), 0, "{{1}}{{G}} is paid exactly");
    Cast { engine, stacked }
}

fn reveals(engine: &Engine<RegistryLookup>) -> Vec<Vec<ObjectId>> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match &entry.event {
            GameEvent::Revealed { player, cards } if *player == P0 => Some(cards.clone()),
            _ => None,
        })
        .collect()
}

fn hand(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(P0)).clone()
}

/// Finds and checks the one printed token, rather than accepting any creature.
fn only_spawn(engine: &Engine<RegistryLookup>) -> ObjectId {
    let tokens: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|object| object.controller == P0 && object.token.is_some())
        })
        .collect();
    assert_eq!(tokens.len(), 1, "exactly one Eldrazi Spawn");
    let token = tokens[0];
    assert_eq!(pt(engine, token), (0, 1));
    assert!(kit::colors_of(engine, token).is_empty(), "colorless");
    assert!(types(engine, token).contains(TypeSet::CREATURE));
    assert!(kit::has_subtype(engine, token, subtypes::creature::ELDRAZI));
    assert!(kit::has_subtype(engine, token, subtypes::creature::SPAWN));
    token
}

/// "Reveal the top four cards ... You may put a permanent card from among them
/// into your hand. Put the rest into your graveyard." Only the creature and
/// land are legal choices; both nonpermanents and the unchosen permanent mill.
#[test]
fn malevolent_rumble_reveals_four_and_takes_at_most_one_permanent() {
    let mut cast = cast(&[
        index::LIGHTNING_BOLT,
        index::GRIZZLY_BEARS,
        index::FOREST,
        index::DIVINATION,
        index::SWAMP,
        index::MOUNTAIN,
    ]);
    pass_until(&mut cast.engine, |engine| {
        matches!(engine.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = cast.engine.pending().clone()
    else {
        panic!("Rumble offers its revealed permanent cards");
    };
    assert_eq!(player, P0);
    assert_eq!(options, vec![cast.stacked[1], cast.stacked[2]]);
    assert_eq!((min, max), (0, 1));
    assert_eq!(prompt, ChoicePrompt::PutIntoHand);
    assert_eq!(reveals(&cast.engine), vec![cast.stacked[..4].to_vec()]);

    assert!(
        cast.engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![cast.stacked[0]],
                },
            )
            .is_err(),
        "an instant is not a permanent card"
    );
    assert!(
        cast.engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![cast.stacked[1], cast.stacked[2]],
                },
            )
            .is_err(),
        "the effect permits at most one card"
    );
    let chosen = cast.stacked[1];
    cast.engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("one offered permanent may be put into hand");
    lk::settle(&mut cast.engine);

    assert_eq!(hand(&cast.engine), vec![chosen]);
    let graveyard = kit::graveyard_of(&cast.engine, P0);
    for card in [cast.stacked[0], cast.stacked[2], cast.stacked[3]] {
        assert!(graveyard.contains(&card), "{card:?} is part of the rest");
    }
    assert!(!graveyard.contains(&chosen));
    assert_eq!(
        kit::library_of(&cast.engine, P0),
        vec![cast.stacked[5], cast.stacked[4]],
        "the uninspected suffix stays in place, bottom first"
    );
    assert_eq!(reveals(&cast.engine), vec![cast.stacked[..4].to_vec()]);

    let spawn = only_spawn(&cast.engine);
    assert_eq!(kit::offered(&cast.engine, spawn), vec![0]);
    kit::activate_object(&mut cast.engine, P0, spawn, 0);
    assert_eq!(lk::pool_of(&cast.engine, P0, ManaColor::Colorless), 1);
    assert_eq!(lk::pool_of(&cast.engine, P0, ManaColor::Green), 0);
    assert_eq!(lk::pool_total(&cast.engine, P0), 1);
    assert!(
        !cast
            .engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&spawn),
        "sacrificing the Spawn is the mana ability's cost"
    );
}

#[test]
fn malevolent_rumble_may_leave_every_revealed_card_in_the_graveyard() {
    let mut cast = cast(&[
        index::FOREST,
        index::LIGHTNING_BOLT,
        index::GRIZZLY_BEARS,
        index::ISLAND,
    ]);
    pass_until(&mut cast.engine, |engine| {
        matches!(engine.pending(), Pending::ChooseCards { .. })
    });
    cast.engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("the permanent card is optional");
    lk::settle(&mut cast.engine);

    assert!(hand(&cast.engine).is_empty());
    let graveyard = kit::graveyard_of(&cast.engine, P0);
    for card in &cast.stacked {
        assert!(graveyard.contains(card));
    }
    assert!(kit::library_of(&cast.engine, P0).is_empty());
    assert_eq!(reveals(&cast.engine), vec![cast.stacked.clone()]);
    only_spawn(&cast.engine);
}

/// No match means no empty question. In particular, an empty library is not
/// a draw and cannot make the controller lose; the token is still created.
#[test]
fn malevolent_rumble_without_permanents_never_asks_an_empty_question() {
    let nonpermanents = [
        index::LIGHTNING_BOLT,
        index::DIVINATION,
        index::LIGHTNING_BOLT,
        index::DIVINATION,
    ];
    for count in [0, 1, 4] {
        let mut cast = cast(&nonpermanents[..count]);
        lk::settle(&mut cast.engine);

        assert!(!matches!(
            cast.engine.pending(),
            Pending::ChooseCards { .. }
        ));
        assert!(!cast.engine.state().players[0].has_lost());
        assert!(hand(&cast.engine).is_empty());
        assert!(kit::library_of(&cast.engine, P0).is_empty());
        let graveyard = kit::graveyard_of(&cast.engine, P0);
        for card in &cast.stacked {
            assert!(graveyard.contains(card));
        }
        let expected = if count == 0 {
            vec![]
        } else {
            vec![cast.stacked.clone()]
        };
        assert_eq!(reveals(&cast.engine), expected);
        only_spawn(&cast.engine);
    }
}

#[test]
fn malevolent_rumble_uses_every_card_in_a_short_library() {
    for count in [1, 3] {
        let mut cast = cast(&vec![index::FOREST; count]);
        pass_until(&mut cast.engine, |engine| {
            matches!(engine.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            options, min, max, ..
        } = cast.engine.pending().clone()
        else {
            panic!("a short library still offers its permanent cards");
        };
        assert_eq!(options, cast.stacked);
        assert_eq!((min, max), (0, 1));
        let chosen = cast.stacked[0];
        cast.engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![chosen],
                },
            )
            .expect("a permanent from a short library is still a legal pick");
        lk::settle(&mut cast.engine);

        assert_eq!(hand(&cast.engine), vec![chosen]);
        assert!(kit::library_of(&cast.engine, P0).is_empty());
        assert_eq!(reveals(&cast.engine), vec![cast.stacked.clone()]);
        only_spawn(&cast.engine);
    }
}

/// The selected commander's move to hand is replaced before anything moves
/// (CR 903.9b). A commander among "the rest" reaches the graveyard first;
/// its owner answers the state-based action afterwards (CR 903.9a).
#[test]
fn malevolent_rumble_offers_command_zone_for_an_unselected_commander() {
    for home in [false, true] {
        let rumble = malevolent_rumble();
        let mut engine = Duel::new(SEED, forest())
            .hand(0, &[rumble, index::LIGHTNING_BOLT, index::DIVINATION])
            .commander(0, &[index::GRIZZLY_BEARS, index::SERRA_ANGEL])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);
        let commanders: Vec<_> = engine.state().commanders[0]
            .iter()
            .map(|commander| commander.object)
            .collect();
        let stacked =
            kit::replace_library(&mut engine, P0, &[index::LIGHTNING_BOLT, index::DIVINATION]);
        for commander in [commanders[1], commanders[0]] {
            engine
                .dev_state_mut(P0)
                .unwrap()
                .move_object(
                    commander,
                    ZoneLocation::Library(P0),
                    baylee_engine::zone::ZonePosition::Top,
                    baylee_engine::event::Cause::Effect,
                )
                .unwrap();
        }
        lk::float(
            &mut engine,
            P0,
            &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        );
        cast_with_floating(&mut engine, P0, rumble);
        pass_until(&mut engine, |engine| {
            matches!(engine.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!("both commanders are permanent cards");
        };
        assert_eq!(options, commanders);
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![commanders[0]],
                },
            )
            .unwrap();

        assert!(matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: baylee_engine::choice::YesNoPrompt::CommanderReplace {
                    card,
                    to_library: false,
                },
                ..
            } if *card == commanders[0]
        ));
        for card in [commanders[0], commanders[1], stacked[0], stacked[1]] {
            assert_eq!(engine.state().object(card).unwrap().zone, Zone::Library);
        }
        engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
        pass_until(&mut engine, |engine| {
            matches!(
                engine.pending(),
                Pending::YesNo {
                    prompt: baylee_engine::choice::YesNoPrompt::CommanderZone { card },
                    ..
                } if *card == commanders[1]
            )
        });

        assert_eq!(hand(&engine), vec![commanders[0]]);
        assert_eq!(
            engine.state().object(commanders[1]).unwrap().zone,
            Zone::Graveyard,
            "the graveyard move is not a hand/library replacement"
        );
        engine.apply(P0, PlayerAction::YesNo(home)).unwrap();
        lk::settle(&mut engine);

        assert_eq!(hand(&engine), vec![commanders[0]]);
        assert_eq!(
            engine.state().object(commanders[1]).unwrap().zone,
            if home { Zone::Command } else { Zone::Graveyard }
        );
        let graveyard = kit::graveyard_of(&engine, P0);
        for card in stacked {
            assert!(graveyard.contains(&card));
        }
        only_spawn(&engine);
    }
}
