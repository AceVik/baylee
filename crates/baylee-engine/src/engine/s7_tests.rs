use super::*;
use crate::choice::CastModeKind;
use baylee_core::ids::{CardIndex, PrintRef};
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
fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn force_of_will() -> CardIndex {
    card_index("956381ba-6d37-4a8a-846c-bad79222dbee")
}
fn counterspell() -> CardIndex {
    card_index("cc187110-1148-4090-bbb8-e205694a39f5")
}
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn mulldrifter() -> CardIndex {
    card_index("24d0f5e7-0d9e-4b76-900e-a7274e80312d")
}
fn cyclonic_rift() -> CardIndex {
    card_index("d75b9c82-1b49-4c3e-a1b5-aeef57d6644b")
}
fn toxic_deluge() -> CardIndex {
    card_index("afaef788-34d1-460b-b884-9d7ae6ddeb18")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

fn preset(
    seed: u64,
    hand0: Vec<CardIndex>,
    bf0: Vec<CardIndex>,
    hand1: Vec<CardIndex>,
    bf1: Vec<CardIndex>,
) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60)
        .map(|i| entry(if i % 2 == 0 { island() } else { forest() }))
        .collect();
    let mk = |hand: Vec<CardIndex>, bf: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck.clone(),
        starting_life: None,
        starting_hand: Some(hand.into_iter().map(entry).collect()),
        starting_battlefield: bf.into_iter().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        dev_mode: false,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(hand0, bf0), mk(hand1, bf1)],
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

fn pass_once(engine: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
}

#[test]
#[allow(clippy::too_many_lines)] // scenario script — step-by-step readability beats extraction
fn force_of_will_pitch_cast_without_mana() {
    // p0 casts a creature; p1 pitches Force of Will (life + exile blue).
    let mut engine = Engine::new(
        &preset(
            31,
            vec![ondu_cleric(), plains(), forest()],
            vec![],
            vec![force_of_will(), counterspell()],
            vec![],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let life_p1 = engine.state().players[1].life;

    // Walk to p0's main, play a land, tap, cast the cleric.
    let mut guard = 0;
    let cleric = loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.lands.is_empty() {
                    engine
                        .apply(
                            player,
                            PlayerAction::PlayLand {
                                card: legal.lands[0],
                            },
                        )
                        .unwrap();
                } else if !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                } else if let Some(&card) = legal.castable.iter().find(|c| {
                    engine
                        .state()
                        .object(**c)
                        .is_some_and(|o| o.card.is_some_and(|d| d.index == ondu_cleric()))
                }) {
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    break card;
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 100, "cleric never cast");
    };

    // p1 pitches FoW: cast mode choice → alternative cost.
    pass_once(&mut engine); // p0 passes after casting
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    let fow = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p1))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == force_of_will()))
        })
        .unwrap();
    assert!(
        legal.castable.contains(&fow),
        "FoW must be castable via pitch"
    );
    engine
        .apply(p1, PlayerAction::CastSpell { card: fow })
        .unwrap();

    // With an empty pool only the pitch alternative is payable, so the
    // wizard auto-selects it and asks for targets directly.

    // Targets: the cleric spell.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected targets, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![cleric]);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();

    // Pitch choice: exile a blue card from hand (Force of Will itself is
    // excluded; the Counterspell is the only blue card).
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected pitch choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (1, 1));
    assert_eq!(options.len(), 1, "only Counterspell is blue");
    let pitch_card = options[0];
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![pitch_card],
            },
        )
        .unwrap();

    // Cost paid: 1 life, card exiled, FoW on stack.
    assert_eq!(engine.state().players[1].life, life_p1 - 1);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&pitch_card)
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Stack)
            .contains(&fow)
    );
}

#[test]
fn mulldrifter_evoke_draws_then_sacrifices() {
    let mut engine = Engine::new(
        &preset(
            32,
            vec![mulldrifter(), island(), island(), island()],
            vec![],
            vec![],
            vec![],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);

    // Play 3 islands over 3 turns, evoke the drifter for {2}{U}.
    let mut cast_done = false;
    let mut guard = 0;
    while !cast_done {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.lands.is_empty() {
                    engine
                        .apply(
                            player,
                            PlayerAction::PlayLand {
                                card: legal.lands[0],
                            },
                        )
                        .unwrap();
                } else if !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                } else if let Some(&card) = legal.castable.iter().find(|c| {
                    engine
                        .state()
                        .object(**c)
                        .is_some_and(|o| o.card.is_some_and(|d| d.index == mulldrifter()))
                }) {
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    // Choose the evoke (alternative) mode if offered.
                    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
                        let alt = options
                            .iter()
                            .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                            .expect("evoke option exists");
                        engine.apply(p0, PlayerAction::ChooseMode(alt)).unwrap();
                    }
                    cast_done = true;
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 100, "drifter never cast");
    }

    // Resolve spell → ETB draw 2 (trigger) → resolve → evoke-sacrifice.
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        let drifter = engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .copied()
            .find(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == mulldrifter()))
            });
        if drifter.is_some() {
            break;
        }
        guard += 1;
        assert!(guard < 40, "drifter never sacrificed");
    }
    // Drew two cards from the ETB trigger (hand: drifter + 3 islands →
    // play 3 lands, cast drifter, +2 drawn = 2).
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 2);
}

#[test]
fn cyclonic_rift_overload_mode_bounces_everything() {
    let mut engine = Engine::new(
        &preset(
            33,
            vec![cyclonic_rift()],
            vec![island(); 7],
            vec![],
            vec![ondu_cleric()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Walk to p0's main phase (mana expires at step end — tap and cast in
    // the same step).
    let mut guard = 0;
    while !matches!(engine.state().turn.phase, Phase::FirstMain) || engine.state().turn.active != p0
    {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        guard += 1;
        assert!(guard < 20);
    }

    // p0 casts the rift with overload ({6}{U} from 6 islands).
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                    continue;
                }
                let rift = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
                engine
                    .apply(player, PlayerAction::CastSpell { card: rift })
                    .unwrap();
                let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
                    panic!("expected modes, got {:?}", engine.pending())
                };
                let overload = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
                    .unwrap_or_else(|| {
                        panic!(
                            "overload mode missing; options: {:?}",
                            options.iter().map(|o| o.kind).collect::<Vec<_>>()
                        )
                    });
                engine
                    .apply(p0, PlayerAction::ChooseMode(overload))
                    .unwrap();
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40);
    }
    pass_once(&mut engine);
    pass_once(&mut engine);
    // The opponent's cleric bounced to their hand (overload hits all
    // opposing nonlands); p0's islands stay (they're lands).
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == ondu_cleric()))
            })
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p1))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == ondu_cleric()))
            })
    );
}

#[test]
fn toxic_deluge_pays_x_life_and_debuffs() {
    let mut engine = Engine::new(
        &preset(
            34,
            vec![toxic_deluge()],
            vec![swamp(), swamp(), swamp()],
            vec![],
            vec![ondu_cleric()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let life_start = engine.state().players[0].life;

    // Walk to p0's main phase first (deluge is sorcery-speed).
    let mut guard = 0;
    while !matches!(engine.state().turn.phase, Phase::FirstMain) || engine.state().turn.active != p0
    {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        guard += 1;
        assert!(guard < 20);
    }

    // Cast with X = 1 (3 swamps: {2}{B} + 1 life).
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                    continue;
                }
                let deluge = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
                engine
                    .apply(player, PlayerAction::CastSpell { card: deluge })
                    .unwrap();
                let Pending::ChooseNumber { .. } = engine.pending().clone() else {
                    panic!("expected X choice, got {:?}", engine.pending())
                };
                engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40);
    }
    assert_eq!(engine.state().players[0].life, life_start - 1);
    pass_once(&mut engine);
    pass_once(&mut engine);
    // The cleric (1/1) dies to -1/-1.
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == ondu_cleric()))
            })
    );
}
