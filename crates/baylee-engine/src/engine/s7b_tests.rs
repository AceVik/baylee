use super::*;
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

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn profane_tutor() -> CardIndex {
    card_index("27a1f42c-0b86-4609-9609-1fa9cab7e7c9")
}
fn ephemerate() -> CardIndex {
    card_index("0fd57894-b917-41c8-a394-360d1d31b236")
}
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
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
) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60)
        .map(|i| entry(if i % 2 == 0 { island() } else { forest() }))
        .collect();
    let mk = |hand: Vec<CardIndex>, bf: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck.clone(),
        sideboard: vec![],
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
        seats: vec![mk(hand0, bf0), mk(hand1, vec![])],
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
#[allow(clippy::too_many_lines)] // scenario script
fn suspend_countdown_casts_for_free_at_zero() {
    let mut engine = Engine::new(
        &preset(51, vec![profane_tutor(), island(), swamp()], vec![], vec![]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);

    // Suspend the tutor (sorcery timing, p0's main phase) — the suspend
    // cost {1}{B} needs mana: play an island and tap it first.
    let mut guard = 0;
    loop {
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
                    continue;
                }
                if !legal.mana_abilities.is_empty() {
                    engine
                        .apply(
                            player,
                            PlayerAction::ActivateManaAbility {
                                source: legal.mana_abilities[0],
                            },
                        )
                        .unwrap();
                    continue;
                }
                // {1}{B} needs two mana: wait for the second land.
                if !legal.suspendable.is_empty() && engine.state().players[0].mana_pool.total() >= 2
                {
                    let card = legal.suspendable[0];
                    engine
                        .apply(player, PlayerAction::Suspend { card })
                        .unwrap();
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
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
            other => panic!(
                "unexpected: {other:?} (turn {}, step {:?}, pool {})",
                engine.state().turn.number,
                engine.state().turn.step,
                engine.state().players[0].mana_pool.total()
            ),
        }
        guard += 1;
        // Turn 2 belongs to p1 (no sorcery timing): the suspend window
        // re-opens on p0's turn 3.
        assert!(guard < 400, "tutor never suspended");
    }
    // The tutor is in exile with 2 time counters.
    let exile = engine.state().zones.list(ZoneLocation::Exile(p0)).clone();
    assert_eq!(exile.len(), 1);
    assert_eq!(
        engine
            .state()
            .object(exile[0])
            .unwrap()
            .counters
            .get(baylee_cards_dsl::CounterKind::Time),
        2
    );

    // Two p0 upkeeps later, the tutor is cast for free and offers the
    // library search.
    let mut searches = 0;
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                assert_eq!(player, p0, "search should be offered to p0");
                searches += 1;
                break;
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
        assert!(guard < 200, "suspended tutor never cast");
    }
    assert_eq!(searches, 1);
}

#[test]
#[allow(clippy::too_many_lines)] // scenario script — step-by-step readability beats extraction
fn ephemerate_rebounds_for_free_at_next_upkeep() {
    let mut engine = Engine::new(
        &preset(
            52,
            vec![ephemerate(), plains()],
            vec![ondu_cleric()],
            vec![],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let cleric = engine.state().zones.list(ZoneLocation::Battlefield)[0];

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

    // Play plains, cast ephemerate on the cleric.
    let mut guard = 0;
    loop {
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
                } else {
                    let eph = engine
                        .state()
                        .zones
                        .list(ZoneLocation::Hand(p0))
                        .iter()
                        .copied()
                        .find(|id| {
                            engine
                                .state()
                                .object(*id)
                                .is_some_and(|o| o.card.is_some_and(|c| c.index == ephemerate()))
                        })
                        .unwrap();
                    engine
                        .apply(player, PlayerAction::CastSpell { card: eph })
                        .unwrap();
                    break;
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
        assert!(guard < 40, "ephemerate never cast");
    }
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected targets, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![cleric]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();

    // Resolve: blink (no change visible) and the spell exiles with rebound.
    let p1 = PlayerId::new(1);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!()
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!()
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let eph = engine
        .state()
        .zones
        .list(ZoneLocation::Exile(p0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == ephemerate()))
        })
        .expect("ephemerate should be exiled with rebound");

    // Next p0 upkeep: the rebound fires — targets are asked again.
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p0);
                assert!(options.contains(&cleric));
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![cleric],
                        },
                    )
                    .unwrap();
                break;
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
        assert!(guard < 100, "rebound never fired");
    }
    let _ = eph;
    let _ = p1;
}
