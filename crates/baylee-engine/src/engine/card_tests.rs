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
        ..
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

fn swamp() -> baylee_core::ids::CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn badlands() -> baylee_core::ids::CardIndex {
    card_index("13ff3222-91cb-4796-a34e-899ed817694c")
}
fn lightning_greaves() -> baylee_core::ids::CardIndex {
    card_index("ca204b66-8d0c-431a-8d34-282f7c2d17da")
}
fn llanowar_elves() -> baylee_core::ids::CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}
fn fellwar_stone() -> baylee_core::ids::CardIndex {
    card_index("95560508-7ac9-4be9-8a3f-3c7d5b52807b")
}
fn an_offer_you_cant_refuse() -> baylee_core::ids::CardIndex {
    card_index("234a734b-ba28-4f1b-9d01-3c3e7d516590")
}
fn dark_ritual() -> baylee_core::ids::CardIndex {
    card_index("53f7c868-b03e-4fc2-8dcf-a75bbfa3272b")
}

/// Activates printed ability `index` of `card`.
#[track_caller]
fn activate(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
    index: u32,
) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, ai)| {
            *ai == index
                && engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .expect("the ability is offered");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ability activates");
}

/// The keywords a battlefield object has *after* the layer system has run,
/// which is the only reading that can see a granted one.
fn keywords(
    engine: &Engine<RegistryLookup>,
    object: baylee_core::ids::ObjectId,
) -> baylee_cards_dsl::KeywordSet {
    engine
        .state()
        .object(object)
        .expect("object exists")
        .characteristics()
        .keywords
}

/// Taps everything that makes mana for `seat`, which is what a player does
/// before casting.
#[track_caller]
fn tap_all_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
}

/// Lightning Greaves: "Equipped creature has haste and shroud." Equipment
/// had two cards in the pool and no engine test at all, so nothing had ever
/// checked the half that matters — that the keywords land on the creature
/// the Equipment is attached to, and not on the Equipment.
#[test]
fn lightning_greaves_grants_both_keywords_to_what_it_is_attached_to() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[forest(), lightning_greaves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves deployed");
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("greaves deployed");
    assert!(
        !keywords(&engine, elves).contains(baylee_cards_dsl::KeywordSet::HASTE),
        "nothing is equipped yet"
    );

    reach_main_phase(&mut engine, p0);
    // Ability 1 is Equip {0}; ability 0 is the static that grants.
    activate(&mut engine, p0, lightning_greaves(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![elves], "the only creature you control");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(elves))
    });
    let kw = keywords(&engine, elves);
    assert!(
        kw.contains(baylee_cards_dsl::KeywordSet::HASTE),
        "equipped creature has haste"
    );
    assert!(
        kw.contains(baylee_cards_dsl::KeywordSet::SHROUD),
        "equipped creature has shroud"
    );
    assert!(
        !keywords(&engine, greaves).contains(baylee_cards_dsl::KeywordSet::SHROUD),
        "the Equipment grants the keywords, it does not keep them"
    );
}

/// Fellwar Stone reads the colours off the lands an *opponent* controls.
/// Reflecting Pool's side of that effect had a test; this side had a card
/// (Exotic Orchard) and none — and the two differ by one comparison, so a
/// sign error there would have produced a Stone that reads your own lands
/// and passed every test in the suite.
#[test]
fn fellwar_stone_reads_the_opponents_lands_and_not_your_own() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(32, forest())
        .battlefield(0, &[forest(), fellwar_stone()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, fellwar_stone(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the opponent's Badlands — your own Forest is not an option"
    );
}

/// An Offer You Can't Refuse: "Counter target noncreature spell. Its
/// controller creates two Treasure tokens." The Treasures go to the player
/// whose spell was countered, which is the whole cost of the card — and the
/// effect resolves *after* the counter, so it has to find that player
/// through a spell that is already a card in a graveyard.
#[test]
fn an_offer_you_cant_refuse_pays_the_countered_spells_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(33, forest())
        .battlefield(0, &[island()])
        .hand(0, &[an_offer_you_cant_refuse()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    // p0 holds; p1 answers with an instant of their own.
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let ritual = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p1))[0];
    engine
        .apply(p1, PlayerAction::CastSpell { card: ritual })
        .unwrap();

    // p0 answers that: tap the Island, counter the Ritual.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    let offer = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: offer })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![ritual], "the only noncreature spell up there");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .unwrap();

    let treasures = |e: &Engine<RegistryLookup>, seat: PlayerId| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .iter()
            .filter(|id| {
                e.state()
                    .object(**id)
                    .is_some_and(|o| o.token.is_some() && o.controller == seat)
            })
            .count()
    };
    pass_until(&mut engine, |e| treasures(e, p1) == 2);
    assert_eq!(
        treasures(&engine, p0),
        0,
        "the Treasures are the countered player's, not the counterer's"
    );
    assert!(
        on_battlefield(&engine, p1, dark_ritual()).is_none(),
        "the Ritual was countered"
    );
}

fn rogue_s_passage() -> baylee_core::ids::CardIndex {
    card_index("f29dc596-2121-4421-8463-15f6c2e8b9b3")
}

/// Rogue's Passage: "{4}, {T}: Target creature can't be blocked this turn."
///
/// `KeywordSet::UNBLOCKABLE` was read by `combat::can_block` and granted by
/// no card in the pool, so the rule had never been exercised from a card.
/// The control is inside the test rather than beside it: seat 0 attacks with
/// two creatures and only one of them was pointed at, so an empty offer —
/// which a blocker that simply could not block would also produce — is not
/// what this asserts.
#[test]
fn rogue_s_passage_takes_its_target_out_of_the_blockers_offer() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(17, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                rogue_s_passage(),
                llanowar_elves(),
                ondu_cleric(),
            ],
        )
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves deployed");
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("cleric deployed");

    // Both of seat 0's creatures are summoning sick on turn 1, so the attack
    // is on turn 3.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(
        !keywords(&engine, elves).contains(baylee_cards_dsl::KeywordSet::UNBLOCKABLE),
        "nothing has been activated yet"
    );

    // Four Forests: the Passage itself is not a basic land, so the CR 305.6
    // shortcut leaves it untapped to pay its own {T}.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rogue_s_passage(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability targets a creature, got {:?}", engine.pending())
    };
    assert!(options.contains(&elves), "any creature is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        keywords(e, elves).contains(baylee_cards_dsl::KeywordSet::UNBLOCKABLE)
    });
    assert!(
        !keywords(&engine, cleric).contains(baylee_cards_dsl::KeywordSet::UNBLOCKABLE),
        "the grant names the target and nothing else"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (elves, baylee_core::ids::Defender::Player(p1)),
                    (cleric, baylee_core::ids::Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    let blockers = loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => break blockers,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while reaching blockers: {other:?}"),
        }
    };
    assert_eq!(blockers.len(), 1, "seat 1 has exactly one creature");
    assert_eq!(
        blockers[0].attackers,
        vec![cleric],
        "the Passage's target is not among the attackers it may be paired with"
    );
}

fn mox_opal() -> baylee_core::ids::CardIndex {
    card_index("de2440de-e948-4811-903c-0bbe376ff64d")
}

/// Karn, the Great Creator +1: "up to **one target** noncreature artifact
/// becomes an artifact creature with power and toughness each equal to its
/// mana value."
///
/// It animated *every* noncreature artifact on every battlefield, because
/// both halves of the sentence were written with the same filter the
/// targeting used — which reads like the same claim and is not. Pointed at a
/// nought-cost artifact it was a one-sided board wipe: everything it touched
/// became a 0/0 and the next state-based check swept it up. Reported from a
/// game as "all lands and artifacts were removed from all fields, except
/// creatures", which is this filter exactly — an artifact *creature* is not
/// a noncreature artifact and was the only thing left standing.
#[test]
fn karn_plus_one_animates_the_target_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(43, forest())
        .battlefield(
            0,
            &[karn_the_great_creator(), mox_opal(), chromatic_lantern()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    let mox = on_battlefield(&engine, p0, mox_opal()).expect("the mox is out");
    let lantern = on_battlefield(&engine, p0, chromatic_lantern()).expect("the lantern is out");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 1,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mox) && options.contains(&lantern),
        "both noncreature artifacts are targetable: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![mox] })
        .unwrap();
    pass_until(&mut engine, |e| on_battlefield(e, p0, mox_opal()).is_none());

    // The mox is a nought-cost artifact, so it animated into a 0/0 and died
    // to a state-based action. That is the rules answer for the card the
    // ability was pointed at, and it is how the fault was noticed at all.
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == mox_opal()))
            }),
        "the target became a 0/0 and was put into its owner's graveyard"
    );
    // And the bystander is untouched: still on the battlefield, and still
    // not a creature. Before the fix it was a 0/0 in the graveyard beside
    // the mox, along with every other noncreature artifact in the game.
    let still = engine
        .state()
        .object(lantern)
        .expect("the lantern was never targeted and is still on the table");
    assert!(
        !still.characteristics().types.intersects(TypeSet::CREATURE),
        "an untargeted artifact was animated too"
    );
    assert!(
        on_battlefield(&engine, p0, chromatic_lantern()).is_some(),
        "and it is still on the battlefield"
    );
}

fn liquimetal_coating() -> baylee_core::ids::CardIndex {
    card_index("f4bdc551-c2eb-4a34-a3e3-b4a017c925af")
}

/// Liquimetal Coating: "{T}: **Target** permanent becomes an artifact in
/// addition to its other types until end of turn."
///
/// The same mistake Karn's `+1` made, found by the lint written for it and
/// worse: the filter reused here was `Filter::Any`, so one tap turned *every
/// permanent in the game* into an artifact — both battlefields, lands
/// included. That is the shape the owner asked about from the other side
/// ("an effect that should only appear for my field"), so the bystander here
/// is the **opponent's** land: an effect pointed at one permanent may not
/// cross the table.
#[test]
fn liquimetal_coating_plates_its_target_and_nobody_elses_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[liquimetal_coating(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let coating = on_battlefield(&engine, p0, liquimetal_coating()).expect("the coating is out");
    let mine = on_battlefield(&engine, p0, forest()).expect("my forest");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their forest");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: coating,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target permanent\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(mine)
            .is_some_and(|o| o.characteristics().types.intersects(TypeSet::ARTIFACT))
    });

    // The target is plated, and the land across the table is a plain Forest.
    assert!(
        !engine
            .state()
            .object(theirs)
            .expect("their forest is still there")
            .characteristics()
            .types
            .intersects(TypeSet::ARTIFACT),
        "the ability reached across the table and plated the opponent's land"
    );
}

/// And the −2 offers the sideboard and exile, never the library.
///
/// Filed beside the wish's own test because the two failures look identical
/// from the client: a dialog full of artifact cards the player did not
/// expect. This one fills the library with the very artifact the wish
/// matches, so a version that read `Library(you)` would offer sixty of them.
#[test]
fn karn_minus_two_never_offers_the_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(44, chromatic_lantern())
        .battlefield(0, &[karn_the_great_creator()])
        .sideboard(0, &[mox_opal()])
        .start();
    keep_mulligans(&mut engine);
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
    let offered = loop {
        match engine.pending().clone() {
            Pending::ChooseCards { options, .. } => break options,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while resolving the wish: {other:?}"),
        }
    };
    for id in &offered {
        let zone = engine.state().object(*id).map(|o| o.zone);
        assert!(
            matches!(
                zone,
                Some(crate::zone::Zone::OutsideGame | crate::zone::Zone::Exile)
            ),
            "the wish offered a card in {zone:?}"
        );
    }
    assert_eq!(offered.len(), 1, "only the mox is outside the game");
}

fn sunken_hollow() -> baylee_core::ids::CardIndex {
    card_index("cd2c90ac-2b04-461c-92f3-939871b6b6a3")
}
/// `Land — Plains Island`, and **nonbasic**: the bystander that separates
/// "an Island" from "a basic land".
fn irrigated_farmland() -> baylee_core::ids::CardIndex {
    card_index("406eabe2-df62-49e2-bb39-c0227509d875")
}

/// Whether the land `seat` just played came in tapped.
#[track_caller]
fn entered_tapped(engine: &Engine<RegistryLookup>, land: baylee_core::ids::ObjectId) -> bool {
    engine
        .state()
        .object(land)
        .expect("the land is on the battlefield")
        .status
        .contains(crate::object::Status::TAPPED)
}

/// Plays `card` out of `seat`'s hand and answers with the object it became.
#[track_caller]
fn play_land(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
) -> baylee_core::ids::ObjectId {
    let land = in_hand(engine, seat, card).expect("the land is in hand");
    engine
        .apply(seat, PlayerAction::PlayLand { card: land })
        .unwrap();
    land
}

/// Sunken Hollow, a battle land: "This land enters tapped unless you control
/// two or more basic lands."
///
/// The condition **counts**, and what it counts is *basic* lands — two
/// things the checkland sentence next to it says neither of. The card was
/// written as a checkland ("unless you control an Island or a Swamp"), which
/// is the same answer on most boards and the wrong one on this one: two
/// Forests are two basic lands and neither is an Island.
#[test]
fn a_battle_land_counts_two_basics_of_any_kind() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[sunken_hollow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hollow = play_land(&mut engine, p0, sunken_hollow());
    assert!(
        !entered_tapped(&engine, hollow),
        "two Forests are two basic lands"
    );
}

/// The other half, and it carries both bystanders the sentence needs.
///
/// One basic of your own is not two; an opponent's basics are not yours; and
/// a land that *prints* the right subtype is not basic — Irrigated Farmland
/// is a `Plains Island` and counts for nothing here, which is the half a
/// filter over subtypes gets exactly backwards.
#[test]
fn a_battle_land_counts_only_your_own_basic_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .battlefield(0, &[island(), irrigated_farmland()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[sunken_hollow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hollow = play_land(&mut engine, p0, sunken_hollow());
    assert!(
        entered_tapped(&engine, hollow),
        "one basic land, a nonbasic Island and two basics across the table"
    );
}

fn deserted_beach() -> baylee_core::ids::CardIndex {
    card_index("f0ec8681-da50-466b-8cdd-1dc710deccd9")
}

/// Deserted Beach, a slow land: "This land enters tapped unless you control
/// two or more **other** lands."
///
/// The word doing the work is "other", and the answer to it is that a
/// permanent's own enters-clause is a replacement applied on the way in, so
/// the land never counts itself. Both halves are one test because the board
/// that proves it is the same board one turn apart: the first Beach arrives
/// with an Island beside it and is the second land on the table, which is
/// two lands and still only *one* other; the second arrives with the Island
/// and the first Beach and comes in untapped. A tapped land counts — the
/// sentence asks what you control, not what is ready.
///
/// The three Forests across the table are the bystander, and they are
/// standing there for the first half, where counting them would have said
/// untapped.
#[test]
fn a_slow_land_counts_the_other_lands_and_never_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(113, forest())
        .battlefield(0, &[island()])
        .battlefield(1, &[forest(), forest(), forest()])
        .hand(0, &[deserted_beach(), deserted_beach()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let first = play_land(&mut engine, p0, deserted_beach());
    assert!(
        entered_tapped(&engine, first),
        "an Island and the Beach itself are not two other lands"
    );

    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_their_main_phase(&mut engine, p0);

    let second = play_land(&mut engine, p0, deserted_beach());
    assert!(
        !entered_tapped(&engine, second),
        "the Island and the first Beach are two other lands"
    );
}

/// The lands whose own enters-tapped-unless filter matches the land printing
/// it, and therefore the exact set that
/// [`a_slow_land_counts_the_other_lands_and_never_itself`] speaks for.
const LANDS_THAT_WOULD_COUNT_THEMSELVES: &[&str] = &[
    "Deathcap Glade",
    "Deserted Beach",
    "Dreamroot Cascade",
    "Haunted Ridge",
    "Overgrown Farmland",
    "Rockfall Vale",
    "Shattered Sanctum",
    "Shipwreck Marsh",
    "Stormcarved Coast",
    "Sundown Pass",
];

/// One line in `controls_at_least` skips the entering permanent, and this is
/// the list of cards that would change if it went.
///
/// Its own comment used to say the list was empty — "nothing in the pool can
/// see the difference", every enters-tapped-unless clause naming something
/// the land is not, and the one cycle that could saying "other" itself. Both
/// halves are wrong. The slow lands print `Filter::And(&[ControlledByYou,
/// LAND])` and are lands, so all ten count themselves; and the cycle the note
/// named as the safe one, Mystic Sanctuary, is a generated stub with no
/// enter-modifier at all. The line is load-bearing for ten implemented cards,
/// and the test above is what plays one.
///
/// The word the ten are missing is one the DSL *can* say — `Filter::Another`,
/// which `eval` reads as `obj.id != this` — and none of them says it, because
/// `landgen` reads "two or more other lands" into a bare count and lets the
/// engine supply the "other". That is a workable division of labour and this
/// is the fence around it: the day a filter here stops matching its own card,
/// or a new cycle starts, the split has to be looked at again rather than
/// discovered by a land that taps for one turn too few.
///
/// Asked through `eval::matches` rather than by reading the filters, because a
/// second implementation of "does this match" is exactly the thing that would
/// agree with itself and not with the engine. Both directions are asserted:
/// an unexpected land is a new cycle, a missing one is a list gone stale.
#[test]
fn the_only_lands_that_would_count_themselves_are_the_slow_ones() {
    let mut clauses = Vec::new();
    let mut behind_the_front_face = Vec::new();
    for def in baylee_cards::all() {
        for (i, face) in def.faces.iter().enumerate() {
            for modifier in face.enter_modifiers {
                let filter = match modifier {
                    baylee_cards_dsl::EnterModifier::TappedUnless(f) => *f,
                    baylee_cards_dsl::EnterModifier::TappedUnlessCount { filter, .. } => *filter,
                    _ => continue,
                };
                if i == 0 {
                    clauses.push((face.name, def.index, filter));
                } else {
                    behind_the_front_face.push(face.name);
                }
            }
        }
    }
    assert!(
        behind_the_front_face.is_empty(),
        "a back face carries an enters-tapped-unless clause, and this sweep \
         puts front faces on the table, so it was never asked about: \
         {behind_the_front_face:?}"
    );
    assert!(
        clauses.len() >= 30,
        "only {} enters-tapped-unless clauses were found; the walk is not \
         reaching the pool",
        clauses.len()
    );

    let p0 = PlayerId::new(0);
    let board: Vec<_> = clauses.iter().map(|(_, index, _)| *index).collect();
    let engine = Duel::new(114, forest()).battlefield(0, &board).start();
    let mut counts_itself = Vec::new();
    for id in engine.state().zones.list(ZoneLocation::Battlefield) {
        let obj = engine.state().object(*id).expect("on the battlefield");
        let Some(card) = obj.card else { continue };
        let Some((name, _, filter)) = clauses.iter().find(|(_, index, _)| *index == card.index)
        else {
            continue;
        };
        // `this` is the entering permanent, which is what `controls_at_least`
        // passes — so a filter that did say `Another` answers `false` here for
        // the same reason the skipped line would have made it moot.
        if crate::eval::matches(filter, engine.state(), obj, p0, obj.id) {
            counts_itself.push(*name);
        }
    }
    counts_itself.sort_unstable();
    assert_eq!(
        counts_itself, LANDS_THAT_WOULD_COUNT_THEMSELVES,
        "the set of lands whose enters-tapped-unless clause matches the land \
         itself has changed; `controls_at_least` skipping the entering \
         permanent is what makes each of them read \"other\", so a new arrival \
         needs a played test and a departure needs this list shortened"
    );
}

fn skyclave_apparition() -> baylee_core::ids::CardIndex {
    card_index("d90af00a-d322-4265-9954-7b1e80702e18")
}

/// Casts the Apparition on p0's first main phase and leaves it on the
/// stack, with `their_board` standing across the table.
fn a_skyclave_over(their_board: &[baylee_core::ids::CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(201, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[skyclave_apparition()])
        .battlefield(1, their_board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a main phase hands priority back");
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = in_hand(&engine, p0, skyclave_apparition()).expect("the Apparition is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("three Plains pay {1}{W}{W}");
    engine
}

/// "Exile **up to one** target ... you don't control" against a board with
/// nothing on it to exile.
///
/// The trigger still goes on the stack. CR 603.3d removes a triggered
/// ability that cannot be given a legal target, and one that requires no
/// target always can be — so what it must not do is *ask*: with an empty
/// option list and a minimum of zero, the only answer is the empty list,
/// and a stop the player cannot influence is not a choice. The engine used
/// to publish `ChooseTargets { options: [], min: 0, max: 0 }` and wait
/// there.
///
/// The second board is what keeps the first honest. A filter that matched
/// nothing at all would pass the first half and read exactly the same, so
/// the same Apparition is put down against a creature it *can* exile and
/// the question has to appear.
#[test]
fn up_to_one_target_with_nothing_to_point_at_is_not_a_question() {
    let p0 = PlayerId::new(0);
    let mut engine = a_skyclave_over(&[forest(), forest()]);
    let mut trigger_stacked = false;
    for _ in 0..20 {
        if stack_is_empty(&engine) {
            break;
        }
        // Something on the stack while the Apparition is already standing is
        // its own enters-trigger: the spell has left, and nothing else at
        // this table triggers at all. Without this the test would read the
        // same on a trigger CR 603.3d had *removed* — an exile that finds
        // nothing to exile does nothing either way, so "nobody was asked" is
        // only half of what is being claimed.
        if on_battlefield(&engine, p0, skyclave_apparition()).is_some() {
            trigger_stacked = true;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("nothing was legal to exile, so nothing was asked: {other:?}"),
        }
    }
    assert!(
        trigger_stacked,
        "the trigger went on the stack, with no targets and no question"
    );
    assert!(stack_is_empty(&engine), "and then resolved");
    assert!(
        on_battlefield(&engine, p0, skyclave_apparition()).is_some(),
        "and the Apparition itself is standing there, trigger and all"
    );

    let mut engine = a_skyclave_over(&[forest(), ondu_cleric()]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, min, .. } = engine.pending().clone() else {
        unreachable!("the loop above stopped on one")
    };
    assert_eq!(min, 0, "\"up to one\" may still decline");
    assert_eq!(
        options,
        vec![on_battlefield(&engine, PlayerId::new(1), ondu_cleric()).expect("their Cleric")],
        "their Cleric is the one thing it may point at"
    );
}

fn eerie_interlude() -> baylee_core::ids::CardIndex {
    card_index("0634091a-a74c-4cea-b6d1-7324a725554a")
}

/// "Exile **any number of** target creatures you control", with none to exile.
///
/// The spell half of what the Apparition's trigger settles above, and it
/// arrives by a different door: `min` is 0 and `max` is 255, so the cast
/// wizard's `max == 0` branch never sees this spell and the *board* is what
/// leaves it with nothing to choose. One legal answer is not a choice, so the
/// spell is cast with no targets rather than the caster being held at
/// `ChooseTargets { options: [], min: 0, max: 255 }` — a stop that can only
/// be answered one way.
#[test]
fn a_spell_that_may_target_any_number_is_not_asked_with_nothing_to_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(202, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[eerie_interlude()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a main phase hands priority back");
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = in_hand(&engine, p0, eerie_interlude()).expect("the Interlude is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("three Plains pay {2}{W}");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no creatures to exile, so no question: {:?}",
        engine.pending()
    );
    assert!(!stack_is_empty(&engine), "and the spell is on the stack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });
}

fn nephalia_drownyard() -> baylee_core::ids::CardIndex {
    card_index("6429b4ed-1845-4643-9a3d-85f7c12f2bba")
}

fn blighted_gorge() -> baylee_core::ids::CardIndex {
    card_index("c2cb0afd-781f-4cfa-b680-ed1edfa81868")
}
fn mountain() -> baylee_core::ids::CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn library_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(seat))
        .len()
}

/// Nephalia Drownyard: "{1}{U}{B}, {T}: Target player mills three cards."
///
/// A player is the only thing this can be pointed at, and the *object* list
/// for such a spec is empty by construction — so an ability `LegalActions`
/// had just offered was refused by `apply` with "no legal targets", the
/// disagreement between two probes this engine treats as the worst kind.
/// Three implemented lands print it and all three were dead: the Drownyard,
/// Duskmantle, House of Shadow and Orzhova, the Church of Deals.
#[test]
fn a_land_that_mills_target_player_can_be_activated() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(203, forest())
        .battlefield(0, &[nephalia_drownyard(), island(), island(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let drownyard = on_battlefield(&engine, p0, nephalia_drownyard()).expect("the Drownyard");
    // Its own tap is part of the ability's cost, so it is the one land that
    // must not be spent on the mana.
    tap_mana_except(&mut engine, p0, drownyard);
    let before = library_size(&engine, p1);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: drownyard,
                ability_index: 1,
            },
        )
        .expect("two Islands and a Swamp pay {1}{U}{B}");
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("the mill asks whose library: {:?}", engine.pending())
    };
    assert!(options.is_empty(), "a seat is not an object");
    assert!(
        player_options.contains(&p1),
        "and the other seat is one of the answers"
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
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });
    assert_eq!(
        library_size(&engine, p1),
        before - 3,
        "three cards off the top of the library that was named"
    );
}

/// Blighted Gorge: "{4}{R}, {T}, Sacrifice this land: it deals 2 damage to
/// any target" — at a table with no creature on it.
///
/// "Any target" is one set spanning objects and players (CR 115.4), and the
/// offer used to count only the objects: with an empty board the ability was
/// withheld, although a player is always there to point at. The activation
/// then had the second half of the same fault, so the two ends of this test
/// are two defects — the ability has to be offered, and it has to go
/// through.
#[test]
fn a_land_that_burns_any_target_reaches_a_face_across_an_empty_board() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(204, forest())
        .battlefield(
            0,
            &[
                blighted_gorge(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let gorge = on_battlefield(&engine, p0, blighted_gorge()).expect("the Gorge");
    tap_mana_except(&mut engine, p0, gorge);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority");
    };
    assert!(
        legal.abilities.contains(&(gorge, 1)),
        "a face is a legal target even with nothing on the battlefield"
    );
    let life = engine.state().players[1].life;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gorge,
                ability_index: 1,
            },
        )
        .expect("five Mountains pay {4}{R}");
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("the damage asks where: {:?}", engine.pending())
    };
    assert!(options.is_empty(), "no creature at the table");
    assert!(player_options.contains(&p1), "but both faces are there");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[1].life,
        life - 2,
        "two damage to the seat that was named"
    );
}

fn wizard_class() -> baylee_core::ids::CardIndex {
    card_index("36f68aa3-9955-46f1-bc87-497f16ef5222")
}

/// Wizard Class ({U}, Class): `{2}{U}: Level 2`, and "when this Class
/// becomes level 2, draw two cards".
///
/// The level-up is an `ActivatedConditional` — it may only be activated at
/// level 1 — and that is the whole point of the test. The condition is a
/// restriction on *activating* the ability (CR 602.5), spent the moment the
/// activation is allowed to begin; what goes on the stack afterwards is an
/// ordinary ability. The engine's resolver did not know that: its match over
/// the ability being resolved listed `Activated` and four others and left
/// `ActivatedConditional` out, so levelling a Class put an ability on the
/// stack that panicked the process as it resolved. Two implemented cards
/// print one of these — this and Luminarch Ascension — and neither had ever
/// had its ability resolved by a test.
#[test]
fn a_class_can_be_levelled_and_the_level_up_resolves() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(919, forest())
        .battlefield(0, &[wizard_class(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is on the table");
    tap_mana_except(&mut engine, p0, class);
    let before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: class,
                ability_index: 1,
            },
        )
        .expect("level 2 is affordable at level 1");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(class)
            .expect("the Class survived its own ability")
            .counters
            .get(baylee_cards_dsl::CounterKind::Level),
        1,
        "one level counter, so the Class is level 2"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        before + 2,
        "becoming level 2 drew two cards"
    );
}

fn bleachbone_verge() -> baylee_core::ids::CardIndex {
    card_index("2b8144a0-08d2-4c28-9fd7-5d90f90105e4")
}

/// Fellwar Stone reads what an opponent's land "could produce" (CR 106.7),
/// and a restriction on activating is not part of that answer.
///
/// Bleachbone Verge adds {B} outright and {W} only while its controller has
/// a Plains or a Swamp. CR 106.7 asks what an ability would produce *if it
/// were to resolve*, says to ignore whether its costs could be paid, and
/// says nothing about activation restrictions — which CR 602.5 puts on
/// beginning the activation, not on the resolution. So the Verge could
/// produce {W} on a board with neither, and this test sets up exactly that
/// board: the opponent's only land is the Verge, and it is the only
/// permanent that could name a colour.
///
/// `Characteristics::from_face` collected the colours from
/// `AbilityDef::Activated { mana_ability: true }` alone, so the Verge
/// offered {B} and nothing else.
#[test]
fn a_conditional_mana_ability_still_says_what_its_land_could_produce() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(733, forest())
        .battlefield(0, &[fellwar_stone()])
        .battlefield(1, &[bleachbone_verge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stone = on_battlefield(&engine, p0, fellwar_stone()).expect("the Stone is on the table");
    // Asked of the offer rather than assumed: `legal.mana_abilities` is the
    // intrinsic path (a basic land's own tap), and a mana ability that runs
    // an effect is offered in `legal.abilities` like any other activation —
    // it skips the stack later, in `start_activation` (CR 605.1).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (_, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == stone)
        .expect("the Stone's mana ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: stone,
                ability_index,
            },
        )
        .expect("a mana ability with no cost but the tap");

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&ManaColor::White),
        "the Verge could produce {{W}} however the condition stands: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black),
        "and {{B}} outright: {options:?}"
    );
}

/// Taps every mana source `seat` has, except the ones printed `skip`.
///
/// [`tap_mana_except`] keeps one object; this keeps a whole printing, which
/// is how a test says "leave the Plains for the instant I am holding".
fn tap_all_mana_but(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    skip: Option<baylee_core::ids::CardIndex>,
) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities {
        let printed = engine
            .state()
            .object(source)
            .and_then(|o| o.card)
            .map(|c| c.index);
        if skip.is_some() && printed == skip {
            continue;
        }
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
}

fn baleful_strix() -> baylee_core::ids::CardIndex {
    card_index("37688720-03de-4eca-a82d-a0afe8d58adc")
}
fn tishanas_tidebinder() -> baylee_core::ids::CardIndex {
    card_index("2993dc7d-723d-4a9b-94bd-4bb02a9f7243")
}

/// A Baleful Strix under a Tishana's Tidebinder that countered its
/// enters-trigger. Answers `(engine, p0, p1, strix)` with the counter
/// resolved and the stack empty.
///
/// p0 keeps a Plains and Swords to Plowshares in reserve, for the half of
/// the sentence that asks what happens when the Tidebinder leaves.
fn a_strix_the_tidebinder_answered() -> (Engine<RegistryLookup>, PlayerId, PlayerId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(23, island())
        .battlefield(0, &[island(), swamp(), plains()])
        .hand(0, &[baleful_strix(), swords_to_plowshares()])
        .battlefield(1, &[island(), island(), island()])
        .hand(1, &[tishanas_tidebinder()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    let strix_card = in_hand(&engine, p0, baleful_strix()).expect("the strix is in hand");
    tap_all_mana_but(&mut engine, p0, Some(plains()));
    engine
        .apply(p0, PlayerAction::CastSpell { card: strix_card })
        .unwrap();

    // Let the Strix resolve; its enters-trigger is what the Tidebinder is
    // here for, so stop as soon as that is on the stack with p1 to answer.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, baleful_strix()).is_some()
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    let strix = on_battlefield(&engine, p0, baleful_strix()).expect("the strix landed");
    let trigger = engine.state().zones.list(crate::zone::ZoneLocation::Stack)[0];

    tap_all_mana_but(&mut engine, p1, None);
    let tide_card = in_hand(&engine, p1, tishanas_tidebinder()).expect("the tidebinder is in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: tide_card })
        .unwrap();

    // The Tidebinder resolves and its own enters-trigger asks for a target.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert!(
        options.contains(&trigger),
        "the strix's enters-trigger was not offered as a target: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![trigger],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, p0, p1, strix)
}

/// The keywords of `object`, as the layers project them.
fn keywords_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> baylee_cards_dsl::KeywordSet {
    engine
        .state()
        .object(object)
        .expect("the object is still there")
        .characteristics()
        .keywords
}

/// Tishana's Tidebinder: "counter up to one target activated or triggered
/// ability. If an ability of an artifact, creature, or planeswalker is
/// countered this way, that permanent loses all abilities for as long as
/// this creature remains on the battlefield."
///
/// The second sentence had never once fired. Both halves read the same
/// target, and the first half removes the countered ability from the arena
/// before the second half looks it up — so it found nothing, registered
/// nothing, and Baleful Strix kept its flying and its deathtouch with a
/// Tidebinder standing over it.
///
/// The keywords are what this asserts because they are what the engine can
/// take away; the rest of the sentence is the `NOT SUPPORTED` note on the
/// card. Both halves are checked: the effect has to be registered *against
/// the Strix*, because a rider aimed at nothing leaves exactly the same
/// keywords standing on a creature that happens to have none.
#[test]
fn tishanas_tidebinder_strips_the_permanent_whose_ability_it_countered() {
    let (engine, _p0, _p1, strix) = a_strix_the_tidebinder_answered();
    let keywords = keywords_of(&engine, strix);
    assert!(
        !keywords.contains(baylee_cards_dsl::KeywordSet::FLYING)
            && !keywords.contains(baylee_cards_dsl::KeywordSet::DEATHTOUCH),
        "the strix kept {keywords:?} after its ability was countered"
    );
    assert!(
        engine.state().effects.iter().any(
            |fx| matches!(fx.filter, crate::effects::EffectFilter::ObjectIs(id) if id == strix)
        ),
        "nothing was registered against the strix, so the keywords went \
         somewhere else or were never there"
    );
}

/// "…for as long as this creature remains on the battlefield." Swords to
/// Plowshares takes the Tidebinder away and the Strix has its flying and its
/// deathtouch back.
///
/// The rider was written `Duration::UntilEndOfTurn`, which is what the card
/// file's header claimed too — and both were wrong in the same direction:
/// the suppression is not a turn's effect, it is the Tidebinder's, and it
/// outlives the turn exactly as long as the Tidebinder does.
#[test]
fn the_strix_takes_its_keywords_back_when_the_tidebinder_leaves() {
    let (mut engine, p0, p1, strix) = a_strix_the_tidebinder_answered();
    let tidebinder = on_battlefield(&engine, p1, tishanas_tidebinder()).expect("it stayed");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana_but(&mut engine, p0, None);
    let stp = in_hand(&engine, p0, swords_to_plowshares()).expect("the sword is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: stp })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&tidebinder),
        "the tidebinder was not a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![tidebinder],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, tishanas_tidebinder()).is_none() && stack_is_empty(e)
    });

    let keywords = keywords_of(&engine, strix);
    assert!(
        keywords.contains(baylee_cards_dsl::KeywordSet::FLYING)
            && keywords.contains(baylee_cards_dsl::KeywordSet::DEATHTOUCH),
        "the tidebinder is gone and the strix is still stripped: {keywords:?}"
    );
}

fn path_to_exile() -> baylee_core::ids::CardIndex {
    card_index("d683d985-9888-4d21-8b5f-69e69ce4a03b")
}

/// Every land `seat` controls, in battlefield order.
fn lands_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<baylee_core::ids::ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == seat && o.characteristics().types.contains(TypeSet::LAND)
            })
        })
        .collect()
}

/// Path to Exile: "Exile target creature. **Its controller** may search their
/// library for a basic land card…"
///
/// The ramp is the half of the card that is not the removal, and it is asked
/// of the seat whose creature just died — never of the seat who cast the
/// spell. `PlayerRel::ControllerOfTarget` is how the card says that, and a
/// site that resolved it through `eval::players` got no seats at all and
/// returned early, so the card shipped as a strictly better Swords to
/// Plowshares that also gave the opponent nothing.
///
/// The assertion therefore names the seat, not just the question: an
/// implementation that offered the search to the *caster* would be exactly as
/// wrong and would pass a test that only counted a `ChooseCards`.
#[test]
fn path_to_exile_offers_the_ramp_to_the_creatures_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[plains()])
        .hand(1, &[path_to_exile()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let victim = on_battlefield(&engine, p0, quiet_creature()).expect("p0's creature");
    let lands_before = lands_of(&engine, p0).len();

    cast_from_hand(&mut engine, p1, path_to_exile());
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!("Path asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "their spell, their target");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("Path points at the creature");

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "the ramp never asked — got {:?}. `OptionalBasicLandSearchFor` \
                 resolves `ControllerOfTarget`, which only `players_of` can \
                 answer.",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the basic-land search, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "the search belongs to the creature's controller, not to the caster"
    );
    assert_eq!((min, max), (0, 1), "\"may search\" — one card at most");
    assert!(
        !options.is_empty(),
        "p0's library is sixty Forests and every one of them is basic"
    );

    // The removal half happened too, and on the right card.
    assert_eq!(
        engine.state().object(victim).map(|o| o.zone),
        Some(crate::zone::Zone::Exile),
        "the creature is exiled, not destroyed"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("p0 takes the land");
    pass_until(&mut engine, |e| lands_of(e, p0).len() > lands_before);
    let fetched = *lands_of(&engine, p0).first().expect("the fetched land");
    assert!(
        engine
            .state()
            .object(fetched)
            .is_some_and(|o| o.status.contains(Status::TAPPED)),
        "\"put that card onto the battlefield tapped\""
    );
}

fn bojuka_bog() -> baylee_core::ids::CardIndex {
    card_index("04b7362d-0490-4cb0-b5d7-2a7732f659ce")
}

/// Bojuka Bog: "When this land enters, exile **target player's** graveyard."
///
/// `PlayerRel::Chosen` — the seat the trigger pointed at — is the other half
/// of the relation `eval::players` cannot answer, and it fails the same
/// silent way: the loop ran over an empty list, the trigger resolved, and the
/// land was a Swamp that cost a land drop.
///
/// Both graveyards are seeded and only one is named, because a fix that
/// resolved `Chosen` as "each player" would empty the caster's own graveyard
/// too and would otherwise pass unnoticed.
#[test]
fn bojuka_bog_exiles_only_the_graveyard_it_targeted() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, forest()).hand(0, &[bojuka_bog()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 3);
    seed_graveyard(&mut engine, p1, 3);
    let mine_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(p0))
        .len();
    assert_eq!(
        mine_before, 3,
        "both graveyards start with something in them"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p1))
            .len(),
        3
    );

    let land = in_hand(&engine, p0, bojuka_bog()).expect("the bog is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .unwrap();

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "the enters trigger never asked for a player: {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected a player choice, got {:?}", engine.pending())
    };
    assert_eq!(
        player_options,
        vec![p0, p1],
        "\"target player\" is anyone at the table (CR 115.1), the caster included"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the bog points at the opponent");

    pass_until(&mut engine, |e| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p1))
            .is_empty()
    });
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Exile(p1))
            .len(),
        3,
        "the cards are exiled, not merely gone"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .len(),
        mine_before,
        "one graveyard was named and only that one is emptied"
    );
}

fn aang_and_katara() -> baylee_core::ids::CardIndex {
    card_index("481c3e14-b670-4fab-aa9f-6ce5b514096d")
}
fn wartime_protestors() -> baylee_core::ids::CardIndex {
    card_index("6557813b-4ee7-4881-a37c-10c8ea097360")
}
fn aminatou() -> baylee_core::ids::CardIndex {
    card_index("3a30089d-cd2d-49be-9b06-7a2454117692")
}

/// The tokens `seat` controls, in arrival order.
fn tokens_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<baylee_core::ids::ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_none() && o.controller == seat)
        })
        .collect()
}

/// Aang and Katara make X Allies at once; Wartime Protestors says
/// "whenever **another Ally** you control enters, put a +1/+1 counter on
/// that creature and it gains haste". Every one of them is an Ally
/// entering, so the rally fires once for each.
///
/// It fired **once for the whole batch**, because the trigger scan broke
/// out of the event loop after the first match. Six tokens arrived, one of
/// them was answered, and the other five were invisible to everything on
/// the board — which is how it was reported: "4 of the tokens disappeared
/// and only one of the two was handled correctly".
///
/// The counted assertion is the counter-test in both directions. One
/// counter on each token fails on the old code (five have none) and would
/// also fail if the loop were made to fire per *permanent* per event, which
/// is the shape that gives N² triggers for N tokens.
#[test]
fn a_rally_trigger_fires_once_for_every_ally_that_entered() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .battlefield(
            0,
            &[
                wartime_protestors(),
                forest(),
                plains(),
                island(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[aang_and_katara()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(tokens_of(&engine, p0).is_empty(), "no tokens yet");

    // Tapping everything for mana is what sets X. The lands go through
    // `mana_abilities` and the Sol Rings do not — an intrinsic land tap
    // and a printed mana ability are two different offers — so both halves
    // are pressed, and only the three artifacts are what Aang and Katara
    // counts.
    tap_all_mana(&mut engine, p0);
    for _ in 0..3 {
        activate(&mut engine, p0, quiet_artifact(), 0);
    }
    let spell = in_hand(&engine, p0, aang_and_katara()).expect("the spell is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("six mana is on the table");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && tokens_of(e, p0).len() == 3
    });

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 3, "one Ally per tapped artifact");
    // Let the three rally triggers resolve.
    pass_until(&mut engine, |e| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .is_empty()
            && matches!(e.pending(), Pending::Priority { .. })
    });

    for (n, token) in tokens.iter().copied().enumerate() {
        let obj = engine
            .state()
            .object(token)
            .expect("the token is still here");
        assert_eq!(
            obj.counters.get(baylee_cards_dsl::CounterKind::P1P1),
            1,
            "token {n} was answered exactly once"
        );
        assert!(
            keywords(&engine, token).contains(baylee_cards_dsl::KeywordSet::HASTE),
            "token {n} gained haste"
        );
    }

    let protestors = on_battlefield(&engine, p0, wartime_protestors()).expect("still there");
    assert_eq!(
        engine
            .state()
            .object(protestors)
            .expect("the source is on the battlefield")
            .counters
            .get(baylee_cards_dsl::CounterKind::P1P1),
        0,
        "the trigger says `another Ally`",
    );
}

/// Aminatou's −1 exiles a permanent you own and returns it — the flicker
/// the owner reported. A tapped land came back tapped.
///
/// CR 400.7: what comes back is a new object with no memory of the old
/// one, and nothing cleared the old one's status, so the tapped bit rode
/// through the exile zone and back. The fix is in
/// [`crate::state::GameState::move_object`], which is the door every zone
/// change goes through, so a bounced creature and a reanimated one are
/// covered by the same three lines.
#[test]
fn a_blinked_permanent_comes_back_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[aminatou(), forest(), plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Tap the land the only way a player can: use it.
    tap_all_mana(&mut engine, p0);
    let land = on_battlefield(&engine, p0, forest()).expect("the forest is on the battlefield");
    assert!(
        engine
            .state()
            .object(land)
            .expect("the land is there")
            .status
            .contains(crate::object::Status::TAPPED),
        "the land is tapped before the flicker",
    );

    activate(&mut engine, p0, aminatou(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(options.contains(&land), "the tapped land is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: vec![],
            },
        )
        .expect("the land is targeted");

    pass_until(&mut engine, |e| {
        e.state()
            .object(land)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            && e.state()
                .zones
                .list(crate::zone::ZoneLocation::Stack)
                .is_empty()
    });
    assert!(
        !engine
            .state()
            .object(land)
            .expect("the land came back")
            .status
            .contains(crate::object::Status::TAPPED),
        "a permanent that changed zones is a new object and enters untapped",
    );
}

fn aether_channeler() -> baylee_core::ids::CardIndex {
    card_index("fb220f46-f8b8-4804-baa4-e7d50b4871f7")
}

/// Casts Aether Channeler off three Islands and hands the engine back
/// standing on its modal trigger's question.
///
/// Four tests share it because the four things worth proving about a modal
/// trigger are one question, two answers and a mode that is not offered —
/// and until the collection arm existed, *none of them was reachable*.
/// `AbilityDef::ModalTriggered` was skipped by both loops in `trigger.rs`,
/// so the ability never became a `PendingTrigger`, never reached the stack
/// and was never asked about: the card resolved, nothing happened, and no
/// error was reported (entry 34).
#[track_caller]
fn a_modal_trigger_asks(
    seed: u64,
    opponent_board: &[baylee_core::ids::CardIndex],
) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, opponent_board)
        .hand(0, &[aether_channeler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, aether_channeler());
    // Priority passes until the spell resolves and its ETB trigger asks.
    for _ in 0..20 {
        if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
            return engine;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "a modal trigger asked nothing and the game moved on — got {:?}. \
                 That is entry 34: the ability is never collected, so the card \
                 resolves and does nothing at all.",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("the modal trigger never asked for its mode")
}

/// The question: all three of Aether Channeler's modes, offered to its
/// controller as the trigger goes on the stack (CR 603.3c).
#[test]
fn a_modal_trigger_offers_every_mode_it_can_legally_choose() {
    let p0 = PlayerId::new(0);
    let engine = a_modal_trigger_asks(53, &[quiet_creature()]);
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the helper returns standing on the question")
    };
    assert_eq!(player, p0, "the trigger's controller chooses the mode");
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            crate::choice::CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![0, 1, 2],
        "a Bird, a bounce and a draw — the bounce is legal because the \
         opponent has a nonland permanent to point it at",
    );
}

/// The first answer: the mode with no targets resolves on its own.
#[test]
fn a_modal_trigger_resolves_the_mode_that_was_chosen() {
    let p0 = PlayerId::new(0);
    let mut engine = a_modal_trigger_asks(59, &[quiet_creature()]);
    let before = tokens_of(&engine, p0).len();
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), before + 1, "mode 0 makes one Bird token");
    assert!(
        engine
            .state()
            .object(*tokens.last().expect("the Bird"))
            .expect("the token exists")
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::FLYING),
        "a 1/1 white Bird with flying",
    );
}

/// The second answer, and the half that `actions.rs` used to skip: a mode
/// that targets asks for *its own* target, not the ability's.
#[test]
fn a_modal_trigger_asks_for_the_targets_of_its_chosen_mode() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_modal_trigger_asks(61, &[quiet_creature()]);
    let elves = on_battlefield(&engine, p1, quiet_creature()).expect("the opponent's creature");
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the bounce mode asked for no target — got {:?}. The mode carries \
             the `TargetReq`, and reading it off the ability finds none.",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert!(
        options.contains(&elves),
        "the opponent's creature is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p1, quiet_creature()).is_some(),
        "\"return another target nonland permanent to its owner's hand\"",
    );
}

/// The mode that cannot be chosen: with nothing else on the battlefield the
/// bounce has no legal target, so CR 603.3c takes it off the list rather
/// than offering a choice that resolves to nothing.
#[test]
fn a_mode_with_no_legal_target_is_not_offered() {
    let engine = a_modal_trigger_asks(67, &[]);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!("the helper returns standing on the question")
    };
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            crate::choice::CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![0, 2],
        "the Bird and the draw; \"another target nonland permanent\" finds \
         nothing on a board of three Islands and the Channeler itself",
    );
}

fn ertai_resurrected() -> baylee_core::ids::CardIndex {
    card_index("3d038f7c-95fa-4b71-8f74-b9b4dd45cde0")
}

/// Ertai Resurrected's second mode, which is the mutant for the collection
/// arm: it is the only place in the pool where
/// `Effect::DrawCardsFor { who: PlayerRel::ControllerOfTarget }` can run at
/// all, and a modal trigger that never fires is a card whose whole printed
/// text is unreachable. "Destroy another target creature or planeswalker.
/// Its controller draws a card."
#[test]
fn ertais_chosen_mode_destroys_and_lets_its_victim_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(71, island())
        .battlefield(0, &[island(), island(), swamp(), swamp()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[ertai_resurrected()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p1, quiet_creature()).expect("the opponent's creature");
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p1))
        .len();
    cast_from_hand(&mut engine, p0, ertai_resurrected());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    // Answered by *position*, and the position is not the mode number here:
    // with an empty stack Ertai's first mode has no spell or ability to
    // counter, so CR 603.3c takes it off the list and "destroy" is offered
    // first. A test that sent `ChooseMode(1)` picked the decline instead —
    // which is what it did before this line existed, and it failed loudly
    // rather than quietly, because the destroy asked for no target.
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            crate::choice::CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![1, 2],
        "destroy and decline; \"counter target spell, activated ability, or \
         triggered ability\" has nothing on an empty stack",
    );
    let destroy = modes
        .iter()
        .position(|m| *m == 1)
        .expect("the destroy mode is offered");
    engine.apply(p0, PlayerAction::ChooseMode(destroy)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the destroy mode asked for no target — got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves),
        "another creature is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, quiet_creature()).is_some(),
        "the targeted creature was destroyed",
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p1))
            .len(),
        hand_before + 1,
        "\"its controller draws a card\" — the *target's* controller, not \
         Ertai's; `PlayerRel::ControllerOfTarget` has never resolved for a \
         trigger before, because no modal trigger ever reached the stack",
    );
}

fn panharmonicon() -> baylee_core::ids::CardIndex {
    card_index("76678885-3674-443d-b9a2-2a460cf6aac0")
}

/// Panharmonicon doubles a modal trigger, and each of the two chooses its
/// own mode.
///
/// The two halves of this pass in one assertion. `trigger_count` reaches
/// `ModalTriggered` because both collection loops read it through
/// `triggered_parts`, so the ability fires twice; and the mode is asked per
/// queue entry rather than per ability, so the two questions can be answered
/// differently — a Bird and a draw, off one Aether Channeler.
#[test]
fn panharmonicon_doubles_a_modal_trigger_and_each_copy_picks_its_own_mode() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(73, island())
        .battlefield(0, &[island(), island(), island(), panharmonicon()])
        .hand(0, &[aether_channeler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    cast_from_hand(&mut engine, p0, aether_channeler());
    let tokens_before = tokens_of(&engine, p0).len();

    // `ChooseMode` is answered by *position*, and the list holds only the
    // modes that can be chosen legally (CR 603.3c), so the position of a
    // mode is looked up rather than assumed — the bounce is on this list,
    // because Panharmonicon is itself a nonland permanent it can point at.
    let mode_at = |engine: &Engine<RegistryLookup>, mode: usize| {
        let Pending::ChooseCastMode { options, .. } = engine.pending() else {
            unreachable!("standing on the mode question")
        };
        options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(m) if m == mode))
            .expect("the mode is offered")
    };

    // The first copy: a Bird.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let token_mode = mode_at(&engine, 0);
    engine
        .apply(p0, PlayerAction::ChooseMode(token_mode))
        .unwrap();
    // The second copy: a draw. Its question is a *separate* one — if the
    // mode were asked once for the ability, this would never appear.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let draw_mode = mode_at(&engine, 2);
    engine
        .apply(p0, PlayerAction::ChooseMode(draw_mode))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        tokens_of(&engine, p0).len(),
        tokens_before + 1,
        "one Bird, from the copy that chose the token mode",
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before,
        "the Channeler left the hand and the draw put one card back",
    );
}

fn umara_raptor() -> baylee_core::ids::CardIndex {
    card_index("a58ee84f-1d9c-4924-b7b1-14a9b2ba3b98")
}

fn solitude() -> baylee_core::ids::CardIndex {
    card_index("dcb9c2a7-ae54-4ddc-a567-640bf4bf4366")
}

/// Every battlefield permanent a seat controls that was printed from `card`.
///
/// [`on_battlefield`] answers the first; this answers all of them, which is
/// what taps *some* of a seat's lands and leaves the rest untapped.
fn all_on_battlefield(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: baylee_core::ids::CardIndex,
) -> Vec<baylee_core::ids::ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// A 1/1 Umara Raptor that put its own rally counter on itself, so the
/// creature standing on the battlefield is a 2/2 and the card it was
/// printed from is not.
///
/// Both tests below need exactly that: a target whose projected power and
/// whose printed power disagree, so the life gained says which of the two
/// the effect read. Only the Islands are tapped for the Raptor — a pool of
/// eight mana pays `{2}` with whatever it likes, and it spent the white the
/// second spell needs.
fn a_two_two_raptor(
    seed: u64,
    spell: baylee_core::ids::CardIndex,
    extra: &[baylee_core::ids::CardIndex],
) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut board = vec![
        island(),
        island(),
        island(),
        plains(),
        plains(),
        plains(),
        plains(),
        plains(),
    ];
    board.extend_from_slice(extra);
    let mut engine = Duel::new(seed, island())
        .battlefield(0, &board)
        .hand(0, &[umara_raptor(), spell])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    for source in all_on_battlefield(&engine, p0, island()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let raptor = in_hand(&engine, p0, umara_raptor()).expect("the Raptor is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: raptor })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor resolved");
    assert_eq!(
        engine
            .state()
            .object(bird)
            .and_then(|o| o.characteristics().power),
        Some(2),
        "the rally trigger put a +1/+1 counter on it",
    );
    for source in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    engine
}

/// CR 608.2g: an effect that needs information about an object no longer in
/// the zone it was expected to be in uses that object's last known
/// information.
///
/// Swords to Plowshares is two effects in one sentence — exile the creature,
/// *then* read its power — so the second half asks about an object the first
/// half moved. It read the printed card and paid one life for a 2/2.
#[test]
fn swords_reads_the_creature_it_exiled_as_it_last_stood() {
    let p0 = PlayerId::new(0);
    let mut engine = a_two_two_raptor(41, swords_to_plowshares(), &[]);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is out");
    let life_before = engine.state().players[0].life;

    let swords = in_hand(&engine, p0, swords_to_plowshares()).expect("the sword is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: swords })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bird],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, umara_raptor()).is_none(),
        "the Raptor was exiled",
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "the power it had on the battlefield, not the 1 its card prints",
    );
}

/// The same sentence on a creature, and the reason the sweep named two cards
/// and not one: Solitude exiles and reads through a *trigger* rather than a
/// spell, which is a second resolution path to the same `Amount`.
#[test]
fn solitudes_trigger_reads_the_creature_it_exiled_as_it_last_stood() {
    let p0 = PlayerId::new(0);
    let mut engine = a_two_two_raptor(42, solitude(), &[]);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is out");
    let life_before = engine.state().players[0].life;

    let incarnation = in_hand(&engine, p0, solitude()).expect("Solitude is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: incarnation })
        .unwrap();
    // Solitude itself targets nothing; the first target question belongs to
    // its enters trigger, and the Raptor is the only other creature.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bird],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, umara_raptor()).is_none(),
        "the Raptor was exiled",
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "the power it had on the battlefield, not the 1 its card prints",
    );
}

fn inspirit_flagship_vessel() -> baylee_core::ids::CardIndex {
    card_index("554df866-3dbb-4811-8573-6033481591aa")
}

/// The other half of CR 608.2g's question, and the one with no mutant: an
/// effect that changes its target and then reads it, while the target is
/// still exactly where the resolution left it.
///
/// Inspirit Flagship Vessel stations a creature — tap it, then take its
/// power in charge counters — and the target never leaves the battlefield,
/// so the read has to be the *live* one. Nothing in the pool tells the two
/// answers apart (a tap changes no power, and these three cards are every
/// reader of `Amount::TargetPower` there is), so this test proves the branch
/// runs rather than that it is the only right one.
#[test]
fn stationing_a_creature_reads_the_power_it_still_has() {
    let p0 = PlayerId::new(0);
    let mut engine = a_two_two_raptor(43, plains(), &[inspirit_flagship_vessel()]);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is out");
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the ship is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == vessel)
        .expect("the station ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bird],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(vessel)
            .map(|o| o.counters.get(baylee_cards_dsl::CounterKind::Charge)),
        Some(2),
        "the Raptor's power on the battlefield, counter and all",
    );
}

fn sheoldred_the_apocalypse() -> baylee_core::ids::CardIndex {
    card_index("34f34409-326d-4994-a0ea-1a69aa278f03")
}

/// A trigger that can find no legal target takes *itself* off the queue and
/// nothing else.
///
/// `collect_triggers` pops the entry it is working on before it asks a
/// synthetic trigger for its target, so the branch that drops a granted
/// trigger with no legal target (CR 603.3d) was popping a second time — and
/// the second pop took whatever was queued behind it, unread and unresolved.
///
/// Wizard Class at level 3 is the only card in the pool that grants a
/// *targeted* trigger, and a Class is an enchantment, so its controller can
/// hold it with no creature anywhere to put the counter on. The draw that
/// fires it fires Sheoldred across the table on the same event, and
/// Sheoldred's is the trigger that was being eaten: the life it takes is the
/// whole assertion.
#[test]
fn a_trigger_that_finds_no_target_takes_only_itself_off_the_queue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![island(); 8];
    board.push(wizard_class());
    let mut engine = Duel::new(9, quiet_artifact())
        .battlefield(0, &board)
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Both levels in one main phase: eight Islands is {2}{U} and {4}{U}
    // exactly, and a mana pool empties at the end of a step, not on a pass.
    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is out");
    tap_mana_except(&mut engine, p0, class);
    activate(&mut engine, p0, wizard_class(), 1);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, p0, wizard_class(), 2);
    pass_until(&mut engine, stack_is_empty);

    let before = engine.state().players[0].life;
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].life,
        before - 2,
        "Sheoldred's trigger was queued behind one that fizzled, and still resolved",
    );
}

/// And the same trigger *answered* takes only itself off the queue.
///
/// The fizzle branch and the answer path are two pops for one queue entry,
/// both of them after the tail pop that already removed it. This is the half
/// a player actually reaches: a creature on the board means the granted
/// trigger has a target, the question is asked, and it was the answer that
/// ate the trigger behind it — so the more a board has going on, the more
/// there is to lose.
#[test]
fn answering_a_granted_triggers_target_takes_only_itself_off_the_queue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![island(); 8];
    board.push(wizard_class());
    board.push(quiet_creature());
    let mut engine = Duel::new(9, quiet_artifact())
        .battlefield(0, &board)
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is out");
    tap_mana_except(&mut engine, p0, class);
    activate(&mut engine, p0, wizard_class(), 1);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, p0, wizard_class(), 2);
    pass_until(&mut engine, stack_is_empty);

    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elves are out");
    let before = engine.state().players[0].life;
    reach_their_main_phase(&mut engine, p1);
    // `walk_to_own_main` and not `reach_their_main_phase`: the draw step on
    // the way asks for the counter's target, which passing cannot answer.
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn came back round"
    );

    assert_eq!(
        engine.state().players[0].life,
        before - 2,
        "Sheoldred's trigger was queued behind one that was answered, and still resolved",
    );
    assert_eq!(
        engine
            .state()
            .object(elves)
            .map(|o| o.counters.get(baylee_cards_dsl::CounterKind::P1P1)),
        Some(1),
        "the granted trigger put its own counter down",
    );
}

/// A two-card draw is two draws, and a draw-watcher fires for both.
///
/// `draw_cards` records one `CardsDrawn { count }` for the whole draw, so an
/// ability that watches draws saw one event and fired once — entry 35's
/// defect in the shape its fix could not see, a batch that is a field rather
/// than a list of events.
///
/// Wizard Class's own level-up draws two cards and Sheoldred, the Apocalypse
/// takes 2 life per card an opponent draws, so the assertion is a **count**
/// in both directions: 2 life is the old bug, 6 would be firing per card and
/// per event both, and 4 is the card.
#[test]
fn a_two_card_draw_fires_a_draw_watcher_twice() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(21, quiet_artifact())
        .battlefield(0, &[wizard_class(), island(), island(), island()])
        .battlefield(1, &[sheoldred_the_apocalypse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is out");
    tap_mana_except(&mut engine, p0, class);
    let before = engine.state().players[0].life;
    activate(&mut engine, p0, wizard_class(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        before - 4,
        "two cards drawn, so Sheoldred took 2 life twice",
    );
}

fn toxic_deluge() -> baylee_core::ids::CardIndex {
    card_index("afaef788-34d1-460b-b884-9d7ae6ddeb18")
}

/// A creature cast after a board-wide debuff resolved is not on its list.
///
/// CR 611.2c: a continuous effect created by a resolving spell or ability
/// that modifies characteristics affects the objects it found when it began,
/// and the set does not change afterwards. Toxic Deluge registered a *live
/// filter* instead — "every creature", asked again on every projection —
/// so a creature cast one priority later entered as a 0/0 and was swept up
/// by the next state-based check, killed by a spell that had already
/// finished resolving.
///
/// Both halves are asserted, because either alone can be passed by a
/// mistake: the opponent's Llanowar Elves was there when the Deluge
/// resolved and dies, and the one cast afterwards stands there at 1/1.
#[test]
fn a_creature_cast_after_a_mass_debuff_is_not_shrunk_by_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), forest()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[toxic_deluge(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Only the swamps: the forest is kept back for the creature, so the
    // test does not depend on which land the payment happens to spend.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        if engine
            .state()
            .object(source)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == swamp()))
        {
            engine
                .apply(p0, PlayerAction::ActivateManaAbility { source })
                .unwrap();
        }
    }
    let deluge = in_hand(&engine, p0, toxic_deluge()).expect("the Deluge is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: deluge })
        .unwrap();
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("expected the X choice, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_none(),
        "the 1/1 that was there when the Deluge resolved took -1/-1 and died",
    );

    cast_from_hand(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, quiet_creature()).is_some()
    });
    let mine = on_battlefield(&engine, p0, quiet_creature())
        .expect("a creature cast after the Deluge survives its arrival");
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the Deluge had already resolved, so it is not one of its creatures",
    );
}

fn darksteel_forge() -> baylee_core::ids::CardIndex {
    card_index("9b3bec05-441f-4fdf-8b51-69fa8613fcd4")
}

/// A permanent that enters under a static grant is projected against it.
///
/// The projection cache is keyed on the *effect table's* generation, and a
/// permanent arriving changes no effect: so the refresh pass at the top of
/// the machine took its early exit, and the newcomer kept the cleared cache
/// `move_object` left it — which reads as the printed card. Darksteel Forge
/// says artifacts you control have indestructible, and a Sol Ring cast into
/// that board had none of it.
///
/// The board is the smaller half of the claim: the Forge is on the
/// battlefield before the game starts, so its static is registered and the
/// generation has been still ever since. Nothing but the arrival is left to
/// account for the difference.
#[test]
fn a_permanent_that_enters_under_a_static_grant_is_projected_against_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .battlefield(0, &[darksteel_forge(), forest()])
        .hand(0, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, quiet_artifact()).is_some()
    });
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Ring resolved");
    assert!(
        engine
            .state()
            .object(ring)
            .expect("the Ring is an object")
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE),
        "the Forge grants indestructible to artifacts that arrive after it too",
    );
}

fn primaris_eliminator() -> baylee_core::ids::CardIndex {
    card_index("7d679591-f8ea-4c4c-ab98-7b9e3438cf57")
}

/// Hyperfrag Round shrinks the creatures of the player it named, and of
/// nobody else.
///
/// "Creatures target player controls get -2/-2 until end of turn" was
/// written as a mode with no target at all and a filter matching every
/// creature on the battlefield — so a 3/2 choosing its own second mode
/// killed itself, the board it had just joined and the opponent's together.
/// The mode targets a player now, and `PumpFilter::controlled_by` is what
/// reads the choice back.
///
/// Three assertions because there are three ways to be wrong: the named
/// seat's creature dies, the caster's does not, and the Eliminator itself
/// is still standing.
#[test]
fn primaris_eliminators_hyperfrag_shrinks_only_the_player_it_named() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(43, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[primaris_eliminator()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my own Elves");

    cast_from_hand(&mut engine, p0, primaris_eliminator());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    let hyperfrag = options
        .iter()
        .position(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(1)))
        .expect("the Hyperfrag Round is offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(hyperfrag))
        .unwrap();

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("Hyperfrag asked for no player — got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1),
        "the opponent is a legal choice for \"target player\"",
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
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_none(),
        "the named player's 1/1 took -2/-2",
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "my own creature is not one of theirs",
    );
    assert!(
        on_battlefield(&engine, p0, primaris_eliminator()).is_some(),
        "and a 3/2 does not kill itself with its own second mode",
    );
}

fn mystical_tutor() -> baylee_core::ids::CardIndex {
    card_index("fb81f95c-70f8-4eb7-8d15-15d0ae23ec03")
}

/// A tutor to the top of the library leaves the card it found on top.
///
/// Mystical Tutor prints "search your library for an instant or sorcery
/// card, reveal it, then shuffle **and put that card on top**", and the
/// search resolution shuffled *after* placing — so the card went on top and
/// was immediately shuffled back into sixty others. Every tutor-to-top in
/// the pool returned a random card.
#[test]
fn a_tutor_to_the_top_leaves_its_card_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(47, counterspell())
        .battlefield(0, &[island()])
        .hand(0, &[mystical_tutor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, mystical_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    let found = *options
        .first()
        .expect("the library is full of Counterspells");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(found),
        "the card the tutor found is the one on top",
    );
}

/// A Spacecraft that stations to 8+ becomes a 5/5, not a corpse.
///
/// "It's an artifact creature at 8+" turns the type on, and the card def
/// carried no power or toughness at all — so the Vessel became a creature
/// with no body and the next state-based check put it into the graveyard.
/// A Spacecraft prints its numbers exactly as a Vehicle does and uses them
/// only once it is stationed.
#[test]
fn a_stationed_spacecraft_becomes_the_creature_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(51, island())
        .battlefield(0, &[inspirit_flagship_vessel()])
        .start();
    keep_mulligans(&mut engine);
    let vessel = on_battlefield(&engine, p0, inspirit_flagship_vessel()).expect("the Vessel");
    assert!(
        !engine
            .state()
            .object(vessel)
            .expect("the Vessel is an object")
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::CREATURE),
        "an unstationed Spacecraft is no creature",
    );

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::put_counters(state, vessel, baylee_cards_dsl::CounterKind::Charge, 8);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert!(
        on_battlefield(&engine, p0, inspirit_flagship_vessel()).is_some(),
        "a stationed Spacecraft is still on the battlefield",
    );
    let chars = engine
        .state()
        .object(vessel)
        .expect("the Vessel is an object")
        .characteristics();
    assert!(
        chars.types.contains(baylee_core::types::TypeSet::CREATURE),
        "at 8+ it is an artifact creature",
    );
    assert_eq!(
        (chars.power, chars.toughness),
        (Some(5), Some(5)),
        "and the body it prints is the body it gets",
    );
}

fn halimar_excavator() -> baylee_core::ids::CardIndex {
    card_index("fd3e37c9-93bf-4f3e-a279-22afbffd8d43")
}

/// Halimar Excavator's rally mills a player the controller *chose*.
///
/// It was written as `PlayerRel::Opponent` with no target requirement at
/// all, so it milled the opponent by construction: the controller could
/// never mill themselves, and a player who could not legally be targeted
/// was milled anyway. The printed line is "target player mills X", and it
/// is not optional — with nobody else legal the controller has to point it
/// at themselves, which is why the requirement's `min` is one.
#[test]
fn halimar_excavator_mills_the_player_it_targeted() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(57, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[halimar_excavator()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let graveyard_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(p0))
        .len();

    cast_from_hand(&mut engine, p0, halimar_excavator());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player_options,
        min,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "either player may be targeted, the controller included",
    );
    assert_eq!(min, 1, "\"target player mills X\" is not optional");

    // Aimed at the controller, which is the half the old card could not do.
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .len(),
        graveyard_before + 1,
        "one Ally on the battlefield, so the player it named mills one card",
    );
}

fn hagra_diabolist() -> baylee_core::ids::CardIndex {
    card_index("5e2c1e0e-0a10-416a-9b50-96ee0cbbc24e")
}
fn vendilion_clique() -> baylee_core::ids::CardIndex {
    card_index("244d4807-0802-41bc-9460-55ac38a28a72")
}
fn loran_of_the_third_path() -> baylee_core::ids::CardIndex {
    card_index("b3d81980-76f2-44e2-b1c9-01e30c726312")
}

/// Hagra Diabolist: "you **may** have target player lose life equal to the
/// number of Allies you control."
///
/// Three cards in the pool said `PlayerRel::Opponent` where their printing
/// says "target player", and this is one of them. That relation is
/// `EachOpponent` in `eval::players`, so the ability drained every opponent
/// at once and could never be pointed at the controller — both invisible in
/// a duel, where "each of them" and "the one you chose" are the same seat.
///
/// The "may" is the target count, the way Sun Titan's "you may return
/// target …" is written here: `min` of nought is the decline.
///
/// Two tests and not two arms of one, because the second cast would want
/// five untapped Swamps a second time and `walk_to_own_main` returns at once
/// when it is already there — a whole turn of passing to prove a second
/// thing the first game has nothing to do with.
#[test]
fn hagra_diabolist_drains_the_player_it_named() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = hagra_on_the_table();
    let (life0, life1) = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );
    let Pending::ChooseTargets {
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the builder stops at the target choice")
    };
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "\"target player\" is every seat, the controller included",
    );
    assert_eq!((min, max), (0, 1), "\"you may\" is the nought in the min");

    // Pointed at the controller's own seat, which the card could not do at
    // all before: `PlayerRel::Opponent` is every opponent and never you.
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let allies = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == p0
                    && o.characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::creature::ALLY)
            })
        })
        .count();
    assert!(allies > 0, "the Diabolist counts itself");
    assert_eq!(
        engine.state().players[0].life,
        life0 - allies as i32,
        "the seat it named lost one life per Ally",
    );
    assert_eq!(
        engine.state().players[1].life,
        life1,
        "and the seat it did not name lost nothing",
    );
}

/// The other half of the "may": declining the target declines the effect.
#[test]
fn hagra_diabolist_may_be_declined() {
    let mut engine = hagra_on_the_table();
    let p0 = PlayerId::new(0);
    let (life0, life1) = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (life0, life1),
        "nobody was named, so nobody lost anything",
    );
}

/// A Hagra Diabolist cast and its rally trigger waiting on a target.
fn hagra_on_the_table() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(19, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[hagra_diabolist()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, hagra_diabolist());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
}

/// Vendilion Clique: "look at **target player's** hand", which is the whole
/// reason the card is played — you point it at yourself to bottom the card
/// you would rather not have drawn and draw again.
///
/// It was written as `PlayerRel::Opponent` with no target at all, so the one
/// thing it is famous for was the one thing it could not do.
#[test]
fn vendilion_clique_may_be_pointed_at_its_own_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[vendilion_clique(), counterspell(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, vendilion_clique());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player_options,
        min,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert!(
        player_options.contains(&p0),
        "the controller is a legal target: {player_options:?}",
    );
    assert!(player_options.contains(&p1), "and so is the opponent");
    assert_eq!(min, 1, "the trigger is not optional; the card choice is");

    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    let library_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();

    // The trigger is on the stack with its target chosen; it resolves when
    // the round of priority after it does.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    // The choice is the *controller's*, whoever's hand it is — "look at
    // target player's hand. **You** may choose a nonland card from it."
    let Pending::ChooseCards {
        player: chooser,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the bottom choice, got {:?}", engine.pending())
    };
    assert_eq!(chooser, p0, "the Clique's controller picks the card");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // One card left the hand for the bottom of the library and one was
    // drawn, so the hand is the size it was and the library is too — and
    // the opponent, who used to be the only seat this could reach, is
    // untouched.
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before,
        "bottomed one and drew one",
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Library(p0))
            .len(),
        library_before,
        "the card went under the library the draw came off",
    );
}

/// Loran of the Third Path: "{T}: You and **target opponent** each draw a
/// card." An opponent, so the controller is not on offer — and *one* of
/// them, which `PlayerRel::Opponent` could not say.
#[test]
fn loran_draws_for_the_one_opponent_she_named() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, plains())
        .battlefield(0, &[loran_of_the_third_path()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let loran = on_battlefield(&engine, p0, loran_of_the_third_path()).expect("Loran deployed");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == loran)
        .expect("Loran's tap ability is offered");
    let (hand0, hand1) = (
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p1))
            .len(),
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();

    let Pending::ChooseTargets {
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        player_options,
        vec![p1],
        "\"target opponent\" leaves the controller out (CR 115.1)",
    );
    assert_eq!((min, max), (1, 1));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand0 + 1,
        "you draw",
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p1))
            .len(),
        hand1 + 1,
        "and so does the opponent you named",
    );
}

fn luminarch_ascension() -> baylee_core::ids::CardIndex {
    card_index("90076bf5-aa9a-4a6e-9035-9aa97fd5561e")
}

/// An Ondu Cleric cast, with its rally trigger asking whether to take the
/// life it offers.
///
/// Two tests over one builder rather than two arms of one, for the reason
/// the Hagra pair has: the second answer wants the same open board the
/// first one spent.
fn a_cleric_asking() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, ondu_cleric());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
}

/// "You may gain life equal to the number of Allies you control" — taken,
/// that is one Ally and one life.
#[test]
fn an_optional_rally_trigger_pays_when_it_is_taken() {
    let p0 = PlayerId::new(0);
    let mut engine = a_cleric_asking();
    let before = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        before + 1,
        "one Ally on the battlefield is one life"
    );
}

/// The other half of the printed "may", and the half the card was written
/// without: declining costs the player nothing and gains them nothing.
#[test]
fn an_optional_rally_trigger_may_be_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = a_cleric_asking();
    let before = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        before,
        "a declined 'may' does nothing at all"
    );
}

/// Luminarch Ascension: a "may" *inside* an intervening-if clause
/// (CR 603.4), which is the shape that put a suspending effect inside a
/// nested branch for the first time.
///
/// The assertion that matters is the count. Running the branch a second
/// time on resume would have asked twice and taken two quest counters —
/// four end steps would have finished the card in two — and nothing in the
/// engine would have complained.
#[test]
fn an_optional_clause_inside_a_condition_is_offered_once_and_taken_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(42, plains())
        .battlefield(0, &[luminarch_ascension()])
        .start();
    keep_mulligans(&mut engine);
    let ascension =
        on_battlefield(&engine, p0, luminarch_ascension()).expect("the enchantment is out");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(ascension)
            .expect("still on the battlefield")
            .counters
            .get(crate::object::CounterKind::Custom(1)),
        1,
        "one offer, one counter"
    );
}

fn jace_the_mind_sculptor() -> baylee_core::ids::CardIndex {
    card_index("7f77a84e-5a4b-4834-aefa-3cecc175ae8e")
}

/// Jace, the Mind Sculptor's +2: "Look at the top card of **target
/// player's** library. **You** may put that card on the bottom of **that
/// player's** library."
///
/// Two players, two roles, and the engine had each of them on the wrong
/// seat. `Effect::ScryFor` handed the `Pending::ChooseCards` to the target,
/// so the *opponent* decided whether to keep their own card — the opposite
/// of what the card says. And `AwaitingOp::Scry` then bottomed the chosen
/// card into `res.controller`'s library, which is not a misplacement so
/// much as a theft: the card came out of one player's library and went into
/// another's.
///
/// Jace is the only card in the pool that uses `ScryFor`, and nothing in
/// the suite had ever activated it, which is how both survived. The two
/// library counts below are what catch the second one; the asked seat
/// catches the first.
#[test]
fn jace_looks_at_a_targets_library_and_the_controller_decides() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(53, forest())
        .battlefield(0, &[jace_the_mind_sculptor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = (library_size(&engine, p0), library_size(&engine, p1));
    let top_of_theirs = *engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p1))
        .last()
        .expect("the opponent has a library");

    let jace = on_battlefield(&engine, p0, jace_the_mind_sculptor()).expect("jace deployed");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: jace,
                ability_index: 0,
            },
        )
        .expect("the +2 activates");
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        panic!("the +2 asks whose library, got {:?}", engine.pending())
    };
    assert!(options.contains(&p1), "the opponent is a legal target");
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();

    // The ability resolves, and the question it raises goes to Jace's
    // controller — never to the player whose library is being looked at.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "\"you may put that card on the bottom\" — you");
    assert_eq!(options, vec![top_of_theirs], "and it is their top card");
    assert_eq!((min, max), (0, 1), "the \"may\" is the zero minimum");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![top_of_theirs],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Library(p1))
            .first()
            .copied(),
        Some(top_of_theirs),
        "the card goes to the bottom of the library it came out of"
    );
    assert_eq!(
        (library_size(&engine, p0), library_size(&engine, p1)),
        before,
        "and no card crossed between the two libraries"
    );
}

fn venser_the_sojourner() -> baylee_core::ids::CardIndex {
    card_index("a8bf8ff8-d924-4fd2-b5ed-05b38f55325a")
}

/// The owner's report: Venser's +2 flickered a Great Divide Guide, and when
/// it came back at the beginning of the end step the Wartime Protestors
/// standing beside it — "whenever **another Ally** you control enters, put a
/// +1/+1 counter on that creature and it gains haste" — said nothing.
///
/// A permanent returning from exile is a permanent *entering the
/// battlefield* (CR 400.7 makes it a new object, and CR 603.6a has the
/// leaves/enters pair), so every watcher on the board is owed its trigger —
/// the one that returned it is not a private arrangement between two
/// objects. Aminatou's immediate flicker is the same claim on the other
/// path and is held by `a_blinked_permanent_comes_back_untapped`; this is
/// the *delayed* one, which runs out of `Engine::process_delayed` rather
/// than out of an effect.
#[test]
fn an_ally_returning_at_the_end_step_still_rallies_the_board() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(53, forest())
        .battlefield(
            0,
            &[
                venser_the_sojourner(),
                wartime_protestors(),
                great_divide_guide(),
                forest(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guide = on_battlefield(&engine, p0, great_divide_guide()).expect("the guide is out");
    assert_eq!(
        engine
            .state()
            .object(guide)
            .expect("the guide is on the battlefield")
            .counters
            .get(baylee_cards_dsl::CounterKind::P1P1),
        0,
        "nothing has rallied yet"
    );

    activate(&mut engine, p0, venser_the_sojourner(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&guide), "the guide is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![guide],
                players: vec![],
            },
        )
        .expect("the guide is targeted");

    // Out to exile first: the return is a separate, delayed thing, and a
    // test that only watched the end result could not tell the two apart.
    pass_until(&mut engine, |e| {
        e.state()
            .object(guide)
            .is_some_and(|o| o.zone == crate::zone::Zone::Exile)
    });

    // Then the end step brings it back, and everything the return sets off
    // has to finish before the assertion.
    pass_until(&mut engine, |e| {
        e.state()
            .object(guide)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            && e.state()
                .zones
                .list(crate::zone::ZoneLocation::Stack)
                .is_empty()
            && matches!(e.pending(), Pending::Priority { .. })
    });

    assert_eq!(
        engine
            .state()
            .object(guide)
            .expect("the guide came back")
            .counters
            .get(baylee_cards_dsl::CounterKind::P1P1),
        1,
        "the Protestors' rally trigger did not see the Ally come back"
    );
    assert!(
        keywords(&engine, guide).contains(baylee_cards_dsl::KeywordSet::HASTE),
        "and the same trigger grants haste until end of turn"
    );
}

/// The types an object has after the layer system has run — the only
/// reading that can see a type a continuous effect added.
fn types(
    engine: &Engine<RegistryLookup>,
    object: baylee_core::ids::ObjectId,
) -> baylee_core::types::TypeSet {
    engine
        .state()
        .object(object)
        .expect("object exists")
        .characteristics()
        .types
}

fn mycosynth_lattice() -> baylee_core::ids::CardIndex {
    card_index("ae1f2ab5-c6a5-4d49-a746-3cb4668bf805")
}
fn brainstorm() -> baylee_core::ids::CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}
fn enlightened_tutor() -> baylee_core::ids::CardIndex {
    card_index("c5229c17-b7be-4b05-b683-f2277edc4849")
}

/// Mycosynth Lattice says "all **permanents** are artifacts", and an instant
/// is not one. It read `Filter::Any` and so reached the stack, where a
/// Brainstorm became an artifact spell — an artifact spell is a permanent
/// spell, and `finalize_spell` put it onto the battlefield and left it
/// there. CR 304.4: an instant cannot enter the battlefield at all.
///
/// Three cards went to the owner's battlefield this way in one game, and
/// Ephemerate's rebound was eaten with them: the card never reached the
/// resolution path that exiles it.
#[test]
fn an_instant_does_not_land_on_the_battlefield_under_mycosynth_lattice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[island(), mycosynth_lattice()])
        .hand(0, &[brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lattice = on_battlefield(&engine, p0, mycosynth_lattice()).expect("lattice deployed");
    let land = on_battlefield(&engine, p0, island()).expect("island deployed");
    assert!(
        types(&engine, land).intersects(baylee_core::types::TypeSet::ARTIFACT),
        "a permanent still is an artifact — the card's own rules text"
    );
    assert!(types(&engine, lattice).intersects(baylee_core::types::TypeSet::ARTIFACT));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let bolt = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    // Sampled with the spell still on the stack and the layer pass behind
    // it — p0 has passed, p1 holds priority. "All permanents" does not reach
    // a spell, and this is the reading `finalize_spell` goes on to make.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .len(),
        1,
        "the spell should still be on the stack here"
    );
    assert!(
        !types(&engine, bolt).intersects(baylee_core::types::TypeSet::ARTIFACT),
        "the Lattice reached the stack: an instant spell became an artifact spell"
    );

    let rest = drive_to_rest(&mut engine, p0);
    assert!(matches!(rest, Rest::Reached), "the duel stalled: {rest:?}");
    assert!(
        on_battlefield(&engine, p0, brainstorm()).is_none(),
        "CR 304.4: an instant card cannot enter the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, brainstorm()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}

/// The other half of the same sentence, and the reason the fix is not only
/// in the card: no type-adding effect may make an instant a permanent, so
/// `TypeSet::is_permanent` answers the question once for every future
/// Lattice. Enlightened Tutor is what noticed — it searches for "an artifact
/// or enchantment card", and every card in the library matched.
#[test]
fn a_library_card_is_not_an_artifact_under_mycosynth_lattice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(42, forest())
        .battlefield(0, &[plains(), mycosynth_lattice()])
        .hand(0, &[enlightened_tutor()])
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
    let tutor = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: tutor })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine
        .apply(PlayerId::new(1), PlayerAction::PassPriority)
        .unwrap();

    // The library is nothing but Forests (the filler), so a correct search
    // finds nothing and the engine asks nothing. Under the old reading every
    // card in it was an artifact and the whole library was on the list.
    assert!(
        !matches!(engine.pending(), Pending::ChooseCards { .. }),
        "the tutor offered a search among Forests: {:?}",
        engine.pending()
    );
    let land = on_battlefield(&engine, p0, plains()).expect("plains deployed");
    assert!(
        types(&engine, land).intersects(baylee_core::types::TypeSet::ARTIFACT),
        "a permanent is still an artifact"
    );
    for &id in engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
    {
        assert!(
            !types(&engine, id).intersects(baylee_core::types::TypeSet::ARTIFACT),
            "a card in the library is not a permanent and gains nothing"
        );
    }
}

fn arid_mesa() -> baylee_core::ids::CardIndex {
    card_index("c5acf2a5-40f4-433d-a74d-1cb56c521464")
}
fn prairie_stream() -> baylee_core::ids::CardIndex {
    card_index("5330e24a-8568-446e-840a-594cd08bd1bc")
}

/// A battle land fetched onto the battlefield still counts basics.
///
/// Prairie Stream enters tapped "unless you control two or more basic
/// lands", and one Arid Mesa activation with a single Plains out put it in
/// untapped. `EnterModifier::Tapped` on a fetched tapland was already held
/// by `s3_tests`; this is the arm beside it, which has to *count* the board
/// rather than write a status, and which no test had ever driven through a
/// search.
#[test]
fn a_fetched_battle_land_counts_the_basics_it_finds() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(43, prairie_stream())
        .battlefield(0, &[plains(), arid_mesa()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mesa = on_battlefield(&engine, p0, arid_mesa()).expect("the fetchland is out");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mesa,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine
        .apply(PlayerId::new(1), PlayerAction::PassPriority)
        .unwrap();

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the fetch's search, got {:?}", engine.pending())
    };
    let found = *options.first().expect("the library is all Prairie Stream");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();

    let obj = engine.state().object(found).expect("the land arrived");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
    assert!(
        obj.status.contains(crate::object::Status::TAPPED),
        "one Plains is not two basic lands"
    );
}

fn orcish_bowmasters() -> baylee_core::ids::CardIndex {
    card_index("ea5103f5-27e0-4eb1-902c-7f34652d6bf3")
}

fn mikokoro() -> baylee_core::ids::CardIndex {
    card_index("a4580a1d-141e-449b-9018-e0258130634b")
}

/// Answers whatever stands between here and the next quiet priority, and
/// says whether a target was ever asked for.
///
/// Written for a *trigger* that targets, which nothing in the pool had until
/// Orcish Bowmasters: a loop that merely tolerates `ChooseTargets` passes
/// whether the question is asked or not, which is how the card sat in the
/// pool marked `Implemented` and pointed at nobody.
fn settle_aiming_at(engine: &mut Engine<RegistryLookup>, face: PlayerId) -> bool {
    let mut asked = false;
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseTargets { player, .. } => {
                asked = true;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![face],
                        },
                    )
                    .expect("a face is a legal target for `any target`");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            _ => break,
        }
    }
    asked
}

/// Orcish Bowmasters: "deals 1 damage to any target. Then amass Orcs 1."
///
/// The card was written with the damage pointed at `PlayerRel::Opponent` and
/// no target requirement on the ability at all, so the arrow was never aimed
/// — the engine reads the *ability's* requirement and the effect's own
/// `target` field is not what it asks about. At a duel that is invisible
/// (there is one opponent, and they were hit either way); at a four-player
/// table it picked one, and it could never hit a creature.
#[test]
fn the_bowmasters_aim_where_their_controller_points() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, island())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[orcish_bowmasters()])
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
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    let before = engine.state().players[1].life;
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("two Swamps pay {1}{B}");

    assert!(
        settle_aiming_at(&mut engine, p1),
        "the enters trigger asks where its arrow goes"
    );
    assert_eq!(
        engine.state().players[1].life,
        before - 1,
        "and the chosen face takes the damage"
    );
}

/// The same ability's other trigger. It is one printed ability with two
/// triggers and the engine has no variant for that, so the card writes it
/// twice — and the second copy was written without the damage, so an
/// opponent's extra draw amassed an Orc and fired no arrow.
///
/// Mikokoro makes both players draw on *this* turn, which is outside the
/// opponent's draw step, so it is never their excepted first draw.
#[test]
fn an_opponents_extra_draw_fires_an_arrow_as_well_as_an_orc() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(12, island())
        .battlefield(0, &[orcish_bowmasters(), mikokoro(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let well = on_battlefield(&engine, p0, mikokoro()).expect("Mikokoro");
    tap_mana_except(&mut engine, p0, well);
    let before = engine.state().players[1].life;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: well,
                ability_index: 1,
            },
        )
        .expect("two Swamps pay the {2}");

    assert!(
        settle_aiming_at(&mut engine, p1),
        "the draw trigger asks where its arrow goes"
    );
    assert_eq!(
        engine.state().players[1].life,
        before - 1,
        "an opponent's extra draw costs them a life, not only a token"
    );
}

fn myr_retriever() -> baylee_core::ids::CardIndex {
    card_index("d07d3be3-f69d-4484-8467-cffd43871788")
}
fn vindicate() -> baylee_core::ids::CardIndex {
    card_index("63c1ac21-e3d8-40c2-8c09-3f31c52992ef")
}

/// Myr Retriever ({2}, 1/1): "When this creature dies, return **another**
/// target artifact card from your graveyard to your hand."
///
/// The word the whole card turns on is `another`, and it is load-bearing in a
/// way no other dies trigger's is: by the time the ability is put on the
/// stack the Myr is itself an artifact card lying in that same graveyard
/// (CR 603.6d, CR 400.7), so a trigger that read "target artifact card" would
/// offer the Myr its own corpse and return it to hand every time — a
/// two-mana artifact that recurs itself forever, which is not the card.
///
/// Both halves are struck, and on the ids the cards have **after** the move:
/// comparing against the Myr's battlefield id would pass however wrong the
/// filter was, because an object changes id when it changes zone.
///
/// The library is filled with an artifact rather than a basic land, which is
/// what gives `seed_graveyard` an artifact card to put there — the Myr needs
/// something legal to point at or the trigger would be removed from the stack
/// for having no legal target, and the interesting assertion would never be
/// reached.
#[test]
fn a_dying_myr_returns_another_artifact_card_and_never_its_own_corpse() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, quiet_artifact())
        .battlefield(0, &[myr_retriever()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "an artifact card is waiting in the graveyard"
    );

    reach_their_main_phase(&mut engine, p1);
    let myr = on_battlefield(&engine, p0, myr_retriever()).expect("the Myr is out");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![myr] })
        .expect("their removal may point at an ordinary creature");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the loop above waited for exactly this")
    };
    let corpse = in_graveyard(&engine, p0, myr_retriever()).expect("the Myr died");
    let other = in_graveyard(&engine, p0, quiet_artifact()).expect("and it is not alone");
    assert!(
        !options.contains(&corpse),
        "`another` keeps the Myr from targeting itself in the graveyard it is \
         now lying in: {options:?}"
    );
    assert!(
        options.contains(&other),
        "and the other artifact card is offered: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![other],
            },
        )
        .expect("the one legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_some(),
        "the artifact card came back to hand"
    );
    assert!(
        in_graveyard(&engine, p0, myr_retriever()).is_some(),
        "and the Myr stayed where it fell"
    );
}

fn ashnods_altar() -> baylee_core::ids::CardIndex {
    card_index("4d18bcba-a346-445e-a182-6cc30b7e066d")
}

/// Ashnod's Altar ({3}): "Sacrifice a creature: Add {C}{C}."
///
/// The card stands at `Coverage::Partial`, and this is the sentence that
/// claim is made of. `cost!(Sacrifice(&Filter::YOUR_CREATURE))` says the
/// printed line exactly; what no engine path can do is suspend an activation
/// to ask *which* creature while the cost is being paid, so `can_afford`
/// refuses a filtered choice cost outright and the ability is never offered.
/// The Altar plays as though the line were not printed, which is what the
/// `Partial` promises a player.
///
/// The test is therefore that nothing is offered, and it is written to
/// **fail** the day that stops being true: when an activation can ask that
/// question, five creatures standing beside the Altar will make this break,
/// and flipping `Partial` to `Implemented` is what closes it. A card whose
/// honesty note nothing checks is a note that outlives its reason —
/// `offer_tests` says no *implemented* card may hide an unofferable ability,
/// and this is the other direction, which nothing said.
///
/// The counter-half is the board: there are creatures to feed it, so an empty
/// offer is the cost refusing and not a table with nothing on it.
#[test]
fn ashnods_altar_offers_nothing_while_a_cost_cannot_ask_which_creature() {
    let seat = PlayerId::new(0);
    let Some((engine, altars)) = arena(ashnods_altar()) else {
        panic!("the Altar is in the pool and stands on a board")
    };
    let fodder = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
            })
        })
        .count();
    assert!(
        fodder >= 2,
        "the board has creatures to sacrifice, so an empty offer below is the \
         cost and not an empty table: {fodder}"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the arena leaves the seat at a quiet main phase")
    };
    let offered = deeds(&legal, &altars);
    assert!(
        offered.is_empty(),
        "a sacrifice cost cannot be chosen during an activation, so the Altar \
         offers nothing at all — if this fires, `pay_cost` learned to ask and \
         Ashnod's Altar is no longer Coverage::Partial: {offered:?}"
    );
}
