use super::*;
use baylee_core::ids::{CardIndex, Defender, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};

struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("card exists")
        .index
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn jace() -> CardIndex {
    card_index("7f77a84e-5a4b-4834-aefa-3cecc175ae8e")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

fn preset(seed: u64, bf0: Vec<CardIndex>) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(island())).collect();
    let mk = |bf: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: bf.into_iter().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(bf0), mk(vec![])],
    }
}

fn keep_mulligans(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

#[test]
fn jace_enters_with_loyalty_and_ticks_up_and_down() {
    let mut engine = Engine::new(&preset(71, vec![jace()]), RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let jace = engine.state().zones.list(ZoneLocation::Battlefield)[0];

    // Jace entered with 3 loyalty (CR 306.5b).
    assert_eq!(
        engine
            .state()
            .object(jace)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::CounterKind::Loyalty),
        3
    );

    // Walk to p0's main and activate +2 (index 0).
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if let Some(&(src, idx)) = legal.abilities.iter().find(|(id, _)| *id == jace) {
                    assert_eq!(idx, 0, "first loyalty ability (+2)");
                    engine
                        .apply(
                            player,
                            PlayerAction::ActivateAbility {
                                source: src,
                                ability_index: idx,
                            },
                        )
                        .unwrap();
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40, "no loyalty ability offered");
    }
    assert_eq!(
        engine
            .state()
            .object(jace)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::CounterKind::Loyalty),
        5
    );

    // Answer the +2's target player choice, then: no second loyalty
    // activation for this walker this turn.
    let p1 = PlayerId::new(1);
    let Pending::ChoosePlayer { player, .. } = engine.pending().clone() else {
        panic!("expected player choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(!legal.abilities.iter().any(|(id, _)| *id == jace));

    // Resolve +2: scry prompt for the chosen player.
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!()
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!()
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let Pending::Arrange {
        prompt: crate::choice::ArrangePrompt::Scry,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a scry arrangement, got {:?}", engine.pending())
    };
}

#[test]
fn walker_at_zero_loyalty_dies() {
    // Jace starts at 3 loyalty; three −1 bounces across three turns → 0 → dies.
    let mut preset = preset(72, vec![jace()]);
    preset.seats[1].starting_battlefield = vec![
        DeckEntry {
            card: ondu_cleric(),
            print: PrintRef::new(0),
        },
        DeckEntry {
            card: ondu_cleric(),
            print: PrintRef::new(0),
        },
        DeckEntry {
            card: ondu_cleric(),
            print: PrintRef::new(0),
        },
    ];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let jace = engine.state().zones.list(ZoneLocation::Battlefield)[0];

    let mut activations = 0;
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if let Some(&(src, _)) = legal
                    .abilities
                    .iter()
                    .find(|(id, idx)| *id == jace && *idx == 2)
                {
                    engine
                        .apply(
                            player,
                            PlayerAction::ActivateAbility {
                                source: src,
                                ability_index: 2,
                            },
                        )
                        .unwrap();
                    activations += 1;
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::ChooseTargets {
                player, options, ..
            } => {
                let cleric_opt = options.first().copied();
                if let Some(t) = cleric_opt {
                    engine
                        .apply(player, PlayerAction::ChooseObjects { objects: vec![t] })
                        .unwrap();
                } else {
                    panic!("no bounce target")
                }
            }
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<_> = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..count as usize].to_vec(),
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        if engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&jace)
        {
            break;
        }
        guard += 1;
        assert!(guard < 400, "jace never died");
    }
    assert!(
        activations >= 3,
        "expected three −1 activations, got {activations}"
    );
}

/// A duel where each seat starts with its own battlefield.
fn preset_both(seed: u64, bf0: Vec<CardIndex>, bf1: Vec<CardIndex>) -> GamePreset {
    let mut preset = preset(seed, bf0);
    preset.seats[1].starting_battlefield = bf1.into_iter().map(entry).collect();
    preset
}

/// Attacking a planeswalker, end to end: the engine offers it as a
/// defender, accepts the declaration, and combat damage comes off its
/// loyalty instead of its controller's life (CR 306.8).
#[test]
fn a_creature_can_attack_a_planeswalker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Engine::new(
        &preset_both(73, vec![ondu_cleric()], vec![jace()]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);

    let find = |engine: &Engine<RegistryLookup>, seat: PlayerId| {
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .find(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.controller == seat)
            })
            .expect("seat has a permanent")
    };
    let cleric = find(&engine, p0);
    let jace = find(&engine, p1);

    // Walk to p0's declare-attackers step.
    let mut guard = 0;
    let defenders = loop {
        match engine.pending().clone() {
            Pending::ChooseAttackers {
                player, defenders, ..
            } if player == p0 => break defenders,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected before combat: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40, "never reached the declare-attackers step");
    };
    assert!(
        defenders.contains(&Defender::Planeswalker(jace)),
        "the opponent's planeswalker was not offered as a defender"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(cleric, Defender::Planeswalker(jace))],
            },
        )
        .unwrap();

    // Let combat damage happen.
    let mut guard = 0;
    while engine
        .state()
        .object(jace)
        .is_some_and(|o| o.counters.get(baylee_cards_dsl::CounterKind::Loyalty) == 3)
    {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected during combat: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40, "combat damage never happened");
    }

    assert_eq!(
        engine
            .state()
            .object(jace)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::CounterKind::Loyalty),
        2,
        "the 1/1 did not take a loyalty counter off"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the attack hit the player as well as the planeswalker"
    );
}

/// The declaration is validated against the engine's own list: a seat
/// cannot attack its own planeswalker, however well-formed the message is.
#[test]
fn a_planeswalker_you_control_is_not_a_legal_defender() {
    let p0 = PlayerId::new(0);
    let mut engine = Engine::new(
        &preset_both(74, vec![ondu_cleric(), jace()], vec![]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let mine: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    let jace = *mine
        .iter()
        .find(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::PLANESWALKER)
            })
        })
        .expect("jace is on the board");
    let cleric = *mine.iter().find(|id| **id != jace).expect("cleric too");

    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::ChooseAttackers {
                player, defenders, ..
            } if player == p0 => {
                assert!(
                    !defenders.contains(&Defender::Planeswalker(jace)),
                    "your own planeswalker was offered as a defender"
                );
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected before combat: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40, "never reached the declare-attackers step");
    }
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(cleric, Defender::Planeswalker(jace))],
                },
            )
            .is_err(),
        "the engine accepted an attack on its declarer's own planeswalker"
    );
}

/// A loyalty ability with nothing to point at is not an action.
///
/// You cannot choose a target that is not there (CR 601.2c, reaching
/// activations through CR 602.2b), so `LegalActions` must not offer one —
/// the same rule the activated abilities already obey, and the loyalty arm
/// was the one that did not. Jace's −1 returns target creature; on a table
/// with no creature on it the +2 and the 0 stay offered and it does not.
#[test]
fn a_loyalty_ability_with_no_legal_target_is_not_offered() {
    let mut engine = Engine::new(&preset(74, vec![jace()]), RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let jace = engine.state().zones.list(ZoneLocation::Battlefield)[0];

    let mut guard = 0;
    let offered = loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                let mine: Vec<u32> = legal
                    .abilities
                    .iter()
                    .filter(|(id, _)| *id == jace)
                    .map(|(_, index)| *index)
                    .collect();
                if !mine.is_empty() {
                    break mine;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40, "no loyalty ability offered");
    };
    assert!(
        offered.contains(&0) && offered.contains(&1),
        "the +2 targets a player and the 0 targets nothing: {offered:?}"
    );
    assert!(
        !offered.contains(&2),
        "the −1 returns target creature and there is no creature: {offered:?}"
    );
}

/// A copied planeswalker's loyalty abilities, answered the way the original's
/// are.
///
/// Spark Double entering as a copy of Karn, the Great Creator has Karn's
/// loyalty abilities as its own (CR 707.2) while its card is still Spark
/// Double. Activating one asks for its target, and the answer has to finish
/// the activation it was asked for. It did not: the answer decided whether it
/// was continuing a loyalty ability by reading the *card's* printed list,
/// where index 1 is no loyalty ability, while the activation had been started
/// from the *object's* own list. So the answer was sent into a second
/// activation, which refused it ("loyalty already used this turn") after the
/// question's continuation had been consumed. The question stayed on the
/// table with nothing behind it, and the next answer panicked the engine
/// ("target plan set"): 1.2 % of the trained AI's self-play games, every one
/// of them a Spark Double copying a walker (Karn, Venser, Elspeth, Teferi).
mod copied_walker {
    use crate::engine::testkit::*;
    use crate::engine::{Engine, Pending, PlayerAction};
    use crate::zone::ZoneLocation;
    use baylee_cards_dsl::CounterKind;
    use baylee_core::ids::{CardIndex, ObjectId, PlayerId};
    use baylee_core::types::TypeSet;

    fn karn() -> CardIndex {
        card_index("a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1")
    }
    fn spark_double() -> CardIndex {
        card_index("8dcb35e5-ae44-455f-86e3-4a77d496ff34")
    }
    fn island() -> CardIndex {
        card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
    }
    /// A noncreature artifact of mana value three, so Karn's +1 makes a
    /// 3/3 of it that lives.
    fn chromatic_lantern() -> CardIndex {
        card_index("539f5396-d99a-417d-a84c-dff7930b5900")
    }

    fn loyalty(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
        engine
            .state()
            .object(id)
            .map_or(0, |o| o.counters.get(CounterKind::Loyalty))
    }

    fn is_creature(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
        engine
            .state()
            .object(id)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
    }

    /// Karn, a Chromatic Lantern and four Islands on seat 0's battlefield,
    /// and a Spark Double cast and entered as a copy of Karn. Answers the
    /// copy and the lantern.
    fn karn_and_his_double(seed: u64) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
        let p0 = PlayerId::new(0);
        let mut engine = Duel::new(seed, island())
            .battlefield(
                0,
                &[
                    karn(),
                    chromatic_lantern(),
                    island(),
                    island(),
                    island(),
                    island(),
                ],
            )
            .hand(0, &[spark_double()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let original = on_battlefield(&engine, p0, karn()).expect("Karn is out");
        let lantern = on_battlefield(&engine, p0, chromatic_lantern()).expect("the lantern");
        cast_from_hand(&mut engine, p0, spark_double());
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![original],
                },
            )
            .expect("the Double copies Karn");
        pass_until(&mut engine, |e| {
            on_battlefield(e, p0, spark_double()).is_some() && stack_is_empty(e)
        });
        let copy = on_battlefield(&engine, p0, spark_double()).expect("the copy entered");
        assert!(
            engine
                .state()
                .object(copy)
                .expect("the copy")
                .characteristics()
                .types
                .contains(TypeSet::PLANESWALKER),
            "the Double is a copy of Karn"
        );
        (engine, copy, lantern)
    }

    /// The one ability on the stack, which must be the copy's.
    fn the_copys_ability(engine: &Engine<RegistryLookup>, copy: ObjectId) -> ObjectId {
        let stack = engine.state().zones.list(ZoneLocation::Stack);
        assert_eq!(stack.len(), 1, "one ability on the stack: {stack:?}");
        let top = stack[0];
        assert_eq!(
            engine
                .state()
                .object(top)
                .and_then(|o| o.ability)
                .map(|a| a.source),
            Some(copy),
            "and it is the copy's"
        );
        top
    }

    #[test]
    fn a_copied_walkers_target_answer_puts_its_ability_on_the_stack() {
        let p0 = PlayerId::new(0);
        let (mut engine, copy, lantern) = karn_and_his_double(301);
        let before = loyalty(&engine, copy);
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: copy,
                    ability_index: 1,
                },
            )
            .expect("the copy's +1 is offered");
        let Pending::ChooseTargets { options, min, .. } = engine.pending().clone() else {
            panic!("the +1 asks for its target, got {:?}", engine.pending())
        };
        assert_eq!(min, 0, "up to one");
        assert!(
            options.contains(&lantern),
            "the lantern is a noncreature artifact"
        );

        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![lantern],
                    players: vec![],
                },
            )
            .expect("the answer to the copy's own question is accepted");
        assert!(
            matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
            "the activation is complete and its activator has priority: {:?}",
            engine.pending()
        );
        let ability = the_copys_ability(&engine, copy);
        assert_eq!(
            engine.state().object(ability).map(|o| o.targets.to_vec()),
            Some(vec![lantern]),
            "pointed at the lantern"
        );
        assert_eq!(loyalty(&engine, copy), before + 1, "the +1 was paid once");

        pass_until(&mut engine, stack_is_empty);
        assert_eq!(pt(&engine, lantern), (3, 3), "the lantern is a 3/3 now");
    }

    #[test]
    fn a_copied_walkers_target_answered_with_nothing_puts_its_ability_on_the_stack() {
        let p0 = PlayerId::new(0);
        let (mut engine, copy, lantern) = karn_and_his_double(302);
        let before = loyalty(&engine, copy);
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: copy,
                    ability_index: 1,
                },
            )
            .expect("the copy's +1 is offered");
        assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![],
                },
            )
            .expect("\"up to one\" answered with none is an answer (CR 115.6)");
        let ability = the_copys_ability(&engine, copy);
        assert!(
            engine
                .state()
                .object(ability)
                .is_some_and(|o| o.targets.is_empty()),
            "targeting nothing"
        );
        assert_eq!(loyalty(&engine, copy), before + 1, "the +1 was paid once");
        pass_until(&mut engine, stack_is_empty);
        assert!(!is_creature(&engine, lantern), "nothing was animated");
    }
}
