//! Behavioral card tests on the shared [`testkit`]: the pattern for the
//! card pool going forward. Each test is deliberately small — the kit
//! carries the duel plumbing, the test carries only the card's rules
//! text as a scenario.

use super::testkit::*;
use super::*;
use baylee_core::mana::ManaColor;

fn forest() -> baylee_core::ids::CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn plains() -> baylee_core::ids::CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn island() -> baylee_core::ids::CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn earth_king_s_lieutenant() -> baylee_core::ids::CardIndex {
    card_index("9da9248d-1201-447f-b6c2-2b64af4f71c4")
}
fn ondu_cleric() -> baylee_core::ids::CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn counterspell() -> baylee_core::ids::CardIndex {
    card_index("cc187110-1148-4090-bbb8-e205694a39f5")
}

/// Earth King's Lieutenant ({G}{W}, 1/1): the ETB puts a +1/+1 counter
/// on each other Ally — here the Ondu Cleric that waited on the board.
#[test]
fn earth_king_s_lieutenant_etb_counters_other_allies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[forest(), plains(), ondu_cleric()])
        .hand(0, &[earth_king_s_lieutenant()])
        .start();
    keep_mulligans(&mut engine);
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("cleric deployed");
    assert_eq!(pt(&engine, cleric), (1, 1));

    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let lieutenant = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: lieutenant })
        .unwrap();

    // The spell resolves, the ETB trigger resolves: the cleric grew.
    pass_until(&mut engine, |e| pt(e, cleric) == (2, 2));
    let lieutenant =
        on_battlefield(&engine, p0, earth_king_s_lieutenant()).expect("lieutenant landed");
    assert_eq!(pt(&engine, lieutenant), (1, 1), "no counter on itself");
}

/// Counterspell: the classic — p0's creature spell never arrives.
#[test]
fn counterspell_counters_a_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(12, forest())
        .battlefield(0, &[forest(), plains()])
        .hand(0, &[ondu_cleric()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let cleric = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: cleric })
        .unwrap();

    // p0 passes; p1 taps both islands and counters the cleric.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p1, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let cs = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p1))[0];
    engine
        .apply(p1, PlayerAction::CastSpell { card: cs })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    // After both pass, the cleric is in the graveyard, not on the board.
    pass_until(&mut engine, |e| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .iter()
            .any(|id| {
                e.state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == ondu_cleric()))
            })
    });
    assert!(on_battlefield(&engine, p0, ondu_cleric()).is_none());
}

fn jin_gitaxias() -> baylee_core::ids::CardIndex {
    card_index("f5daadc1-98ff-480a-82bb-fe7bfaa7b60e")
}
fn swords_to_plowshares() -> baylee_core::ids::CardIndex {
    card_index("b1544f21-7e98-461b-aed5-e748b0168c52")
}

/// Jin-Gitaxias, Progress Tyrant: "copy that spell. You may choose new
/// targets for the copy." The copy starts on the original's target, and its
/// controller is asked whether to move it — here they do, so one Swords to
/// Plowshares exiles two creatures.
#[test]
fn jin_gitaxias_copy_may_be_given_a_new_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(21, forest())
        .battlefield(0, &[plains(), jin_gitaxias()])
        .hand(0, &[swords_to_plowshares()])
        .battlefield(1, &[ondu_cleric(), earth_king_s_lieutenant()])
        .start();
    keep_mulligans(&mut engine);
    let cleric = on_battlefield(&engine, p1, ondu_cleric()).expect("cleric deployed");
    let lieutenant =
        on_battlefield(&engine, p1, earth_king_s_lieutenant()).expect("lieutenant deployed");

    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let swords = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: swords })
        .unwrap();

    // The spell's own target, chosen at cast time: p1's cleric.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the spell's target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&cleric), "the cleric is targetable");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();

    // Walk to the copy's re-choice, answering anything the trigger asks on
    // the way (its own target is the spell that was cast).
    let options = options_offered_including(&mut engine, lieutenant);
    assert!(
        options.contains(&cleric) && options.contains(&lieutenant),
        "every legal creature is offered, not just the original target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lieutenant],
            },
        )
        .unwrap();

    // Original exiles the cleric, the retargeted copy exiles the lieutenant.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, ondu_cleric()).is_none()
            && on_battlefield(e, p1, earth_king_s_lieutenant()).is_none()
    });
}

fn storm_of_saruman() -> baylee_core::ids::CardIndex {
    card_index("cf5f4860-e805-46a3-9352-a2c583e33403")
}

/// Storm of Saruman: the copy trigger fires on the *second* spell, not the
/// first, and the copy it makes is not itself a cast spell — otherwise each
/// copy would be another "second spell" and the trigger would never stop.
#[test]
fn storm_of_saruman_copies_only_the_second_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(22, forest())
        .battlefield(0, &[plains(), plains(), storm_of_saruman()])
        .hand(0, &[swords_to_plowshares(), swords_to_plowshares()])
        .battlefield(1, &[ondu_cleric(), earth_king_s_lieutenant()])
        .start();
    keep_mulligans(&mut engine);
    let cleric = on_battlefield(&engine, p1, ondu_cleric()).expect("cleric deployed");
    let lieutenant =
        on_battlefield(&engine, p1, earth_king_s_lieutenant()).expect("lieutenant deployed");

    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }

    // First spell: no trigger, so it simply resolves and exiles the cleric.
    let first = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: first })
        .unwrap();
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected the spell's target choice, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, ondu_cleric()).is_none()
    });

    // Second spell: the trigger copies it, and the copy may be retargeted.
    let second = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: second })
        .unwrap();
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected the spell's target choice, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lieutenant],
            },
        )
        .unwrap();

    let offered = options_offered_including(&mut engine, lieutenant);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![offered[0]],
            },
        )
        .unwrap();

    // The copy resolves and the trigger does not fire again: a copy is put on
    // the stack, never cast, so it is not a third spell.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, earth_king_s_lieutenant()).is_none()
    });
}

fn karn_the_great_creator() -> baylee_core::ids::CardIndex {
    card_index("a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1")
}
fn chromatic_lantern() -> baylee_core::ids::CardIndex {
    card_index("539f5396-d99a-417d-a84c-dff7930b5900")
}

/// Karn, the Great Creator −2: "reveal an artifact card you own from outside
/// the game ... put that card into your hand."
///
/// Also the regression test for the sideboard itself: those cards must be
/// reachable by the wish and absent from the library, which is where they
/// used to end up.
#[test]
fn karn_minus_two_pulls_an_artifact_from_outside_the_game() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[karn_the_great_creator()])
        .sideboard(0, &[chromatic_lantern()])
        .start();
    keep_mulligans(&mut engine);

    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0));
    assert!(
        !library.iter().any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == chromatic_lantern()))
        }),
        "the sideboard was shuffled into the library"
    );

    reach_main_phase(&mut engine, p0);
    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 2,
            },
        )
        .unwrap();

    // The ability goes on the stack; the wish is offered when it resolves.
    let mut offered = None;
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                options, min, max, ..
            } => {
                assert_eq!((min, max), (0, 1), "the wish is optional and singular");
                offered = Some(options);
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while resolving the wish: {other:?}"),
        }
    }
    let offered = offered.expect("the wish offered the sideboard");
    assert_eq!(
        offered.len(),
        1,
        "only the artifact outside the game qualifies"
    );

    let wanted = offered[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wanted],
            },
        )
        .unwrap();
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .contains(&wanted),
        "the wished-for card is in hand"
    );
}

/// CR 601.2c: a spell whose mandatory target has no legal choice cannot be
/// cast at all, so it must not be offered. Counterspell with an empty stack
/// is the clean case — offering it hands a human a button that only errors,
/// and an agent an action it will pick again on every pass, because failing
/// changes nothing about the state.
#[test]
fn a_spell_with_no_legal_target_is_not_offered() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(51, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.is_empty(),
        "counterspell was offered with nothing on the stack to counter"
    );
}

fn abraded_bluffs() -> baylee_core::ids::CardIndex {
    card_index("ca7d093c-0533-493f-9ad3-8af30118fbfc")
}

/// Abraded Bluffs: "When this land enters, it deals 1 damage to target
/// opponent." Two things are being asserted, and the card was broken on
/// both until `TargetSpec::AnyOpponent` existed. A trigger may point at a
/// *player* at all — before this, `eval::target_options` returned an empty
/// list for a player spec and CR 603.3d quietly binned the trigger — and
/// "target opponent" is a choice over the opponents only (CR 115.1), so the
/// controller must not be among the options.
#[test]
fn an_enters_trigger_can_burn_target_opponent_but_never_its_controller() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(23, forest()).hand(0, &[abraded_bluffs()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == abraded_bluffs())
        })
        .expect("the land is in hand");
    let before = engine.state().players[1].life;
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .unwrap();

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for the trigger: {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(
        options.is_empty(),
        "the damage points at a player, not an object"
    );
    assert_eq!(
        player_options,
        vec![p1],
        "only the opponent is a legal target"
    );
    assert_eq!((min, max), (1, 1));

    // The controller is not on offer, and saying so anyway is refused.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p0],
                },
            )
            .is_err(),
        "a card that says `target opponent` must not be pointable at its controller"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().players[1].life < before);
    assert_eq!(engine.state().players[1].life, before - 1);
}

fn treetop_village() -> baylee_core::ids::CardIndex {
    card_index("b53f216d-1592-4eee-b204-502a805fbc8c")
}

/// Treetop Village: "{1}{G}: This land becomes a 3/3 green Ape creature
/// with trample until end of turn. It's still a land."
///
/// The transcoder writes that sentence as five continuous effects, one per
/// layer, and five plausible literals are not a working card — this is the
/// test that the composition is right: the land is a 3/3 creature *and*
/// still a land, so it can attack and still make mana.
#[test]
fn an_animated_land_becomes_a_creature_and_stays_a_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .battlefield(0, &[treetop_village(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let village = on_battlefield(&engine, p0, treetop_village()).expect("village deployed");
    assert!(
        !engine
            .state()
            .object(village)
            .expect("village exists")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "a land is not a creature before anyone pays for it"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // Index 0 is the mana ability, which `legal.abilities` lists as well;
    // index 1 is the printed "{1}{G}: … becomes a 3/3 Ape".
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, index)| *id == village && *index == 1)
        .expect("the animate ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(village)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
    });

    let types = engine
        .state()
        .object(village)
        .expect("village exists")
        .characteristics()
        .types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND), "it's still a land");
    assert_eq!(pt(&engine, village), (3, 3));
    assert!(
        engine
            .state()
            .object(village)
            .expect("village exists")
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::TRAMPLE),
        "with trample"
    );
}

fn great_divide_guide() -> baylee_core::ids::CardIndex {
    card_index("79e69a91-d580-47fb-be76-1e32c50d2fa0")
}

/// Great Divide Guide grants "{T}: Add one mana of any color" to each land and
/// Ally its controller has — and it is an Ally, so it grants the ability to
/// itself.
///
/// A *granted* mana ability is offered in `LegalActions::mana_abilities`
/// alongside the CR 305.6 shortcut, and until now it could not be taken from
/// there: `ActivateManaAbility` went straight to `intrinsic_mana`, which
/// answers only for a land with one basic type, so the engine refused an
/// action it had just listed. Every caller reads that list the same way — the
/// house AI sends `ActivateManaAbility { source: legal.mana_abilities[0] }`
/// outright — so the list has to mean one thing.
#[test]
fn a_granted_mana_ability_is_activatable_the_way_it_is_offered() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(63, forest())
        .battlefield(0, &[great_divide_guide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guide = on_battlefield(&engine, p0, great_divide_guide()).expect("the guide is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the main phase grants priority");
    };
    assert!(
        legal.mana_abilities.contains(&guide),
        "the guide grants itself a mana ability and the engine offers it"
    );
    assert!(
        !legal.lands.contains(&guide),
        "and it is not a land, which is the whole point: it has no intrinsic mana"
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: guide })
        .expect("an offered mana ability is activatable");

    // "One mana of any color" asks which — the ability resolved rather than
    // erroring, which is the claim.
    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("any-colour mana asks a colour, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "and asks the seat that tapped it");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("the colour is the ability's own choice");

    // Not erroring is only half of it. The synthetic index reaches
    // `start_activation`, which is what pays the cost — if it ever skipped
    // that, a granted `{T}` ability would be infinite mana and this is where
    // that has to fail.
    assert!(
        engine
            .state()
            .object(guide)
            .expect("the guide is still there")
            .status
            .contains(crate::object::Status::TAPPED),
        "paying {{T}} left it tapped"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "and it is the colour that was named"
    );
}
