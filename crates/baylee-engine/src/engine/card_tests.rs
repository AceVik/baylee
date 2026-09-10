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
