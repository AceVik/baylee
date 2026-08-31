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
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn harabaz_druid() -> CardIndex {
    card_index("ead985ec-f29f-4a3b-b8b1-061142cc5bd1")
}
fn panharmonicon() -> CardIndex {
    card_index("76678885-3674-443d-b9a2-2a460cf6aac0")
}
fn elesh_norn() -> CardIndex {
    card_index("5ade11c0-41dd-4b6a-9f5b-c5903a3a0d7f")
}
fn doubling_season() -> CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}
fn maskwood_nexus() -> CardIndex {
    card_index("9b2cdbed-c733-409b-b0e4-2c8960c25111")
}
fn skyclave() -> CardIndex {
    card_index("d90af00a-d322-4265-9954-7b1e80702e18")
}
fn swords() -> CardIndex {
    card_index("b1544f21-7e98-461b-aed5-e748b0168c52")
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
        .map(|i| entry(if i % 2 == 0 { forest() } else { plains() }))
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

/// Drives the game until `until` holds: plays lands/taps for p0 when
/// possible, passes otherwise.
fn drive_until(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    mut on_castable: impl FnMut(&mut Engine<RegistryLookup>, ObjectId),
    mut until: impl FnMut(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..400 {
        if until(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, legal } => {
                if player == p0 && !legal.lands.is_empty() {
                    engine
                        .apply(
                            player,
                            PlayerAction::PlayLand {
                                card: legal.lands[0],
                            },
                        )
                        .unwrap();
                } else if player == p0 && !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                } else if player == p0 && !legal.castable.is_empty() {
                    on_castable(engine, legal.castable[0]);
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
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
    }
    panic!("drive_until exhausted without reaching the condition");
}

#[test]
fn panharmonicon_doubles_rally_trigger() {
    let mut engine = Engine::new(
        &preset(
            21,
            vec![harabaz_druid()],
            vec![panharmonicon(), ondu_cleric(), forest(), forest()],
            vec![],
            vec![],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let life_start = engine.state().players[0].life;
    drive_until(
        &mut engine,
        p0,
        |engine, card| {
            engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
        },
        |e| e.state().players[0].life == life_start + 2,
    );
    // One Ally ETB → rally triggers twice with Panharmonicon.
    assert_eq!(engine.state().players[0].life, life_start + 2);
}

#[test]
fn elesh_norn_suppresses_opponent_and_doubles_own() {
    // p1: Elesh Norn + Ondu Cleric + forests. p0 plays an Ally (druid).
    let mut engine = Engine::new(
        &preset(
            22,
            vec![harabaz_druid()],
            vec![forest(), forest()],
            vec![harabaz_druid()],
            vec![elesh_norn(), ondu_cleric(), forest(), forest()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let life_p1_start = engine.state().players[1].life;

    // p0 plays the druid → p1's cleric rally is SUPPRESSED.
    let druid_on_bf = |e: &Engine<RegistryLookup>| {
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .any(|id| {
                e.state().object(*id).is_some_and(|o| {
                    o.card
                        .is_some_and(|c| c.index == harabaz_druid() && o.controller == p0)
                })
            })
    };
    drive_until(
        &mut engine,
        p0,
        |engine, card| {
            engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
        },
        |e| druid_on_bf(e),
    );
    // A full priority round after the ETB to let (suppressed) triggers resolve.
    drive_until(
        &mut engine,
        p0,
        |engine, _| {
            let Pending::Priority { player, .. } = engine.pending().clone() else {
                return;
            };
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        },
        |e| e.state().turn.active == p1,
    );
    assert_eq!(
        engine.state().players[1].life,
        life_p1_start,
        "opponent rally must be suppressed by Elesh Norn"
    );

    // Now p1 plays its druid → p1's rally triggers TWICE (multiplier).
    let life_p1_before = engine.state().players[1].life;
    drive_until(
        &mut engine,
        p1,
        |engine, card| {
            engine.apply(p1, PlayerAction::CastSpell { card }).unwrap();
        },
        |e| e.state().players[1].life == life_p1_before + 2,
    );
    assert_eq!(engine.state().players[1].life, life_p1_before + 2);
}

#[test]
fn doubling_season_doubles_token_creation() {
    let mut engine = Engine::new(
        &preset(
            23,
            vec![],
            vec![
                doubling_season(),
                maskwood_nexus(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
            vec![],
            vec![],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let nexus = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == maskwood_nexus()))
        })
        .unwrap();
    // Tap 3 forests + the Nexus ability.
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } => {
                for source in legal.mana_abilities.clone() {
                    engine
                        .apply(player, PlayerAction::ActivateManaAbility { source })
                        .unwrap();
                }
                let Pending::Priority { legal, .. } = engine.pending().clone() else {
                    panic!()
                };
                if legal.abilities.contains(&(nexus, 1)) {
                    engine
                        .apply(
                            player,
                            PlayerAction::ActivateAbility {
                                source: nexus,
                                ability_index: 1,
                            },
                        )
                        .unwrap();
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 30);
    }
    // Resolve the ability: TWO shapeshifter tokens instead of one.
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
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
            other => panic!("unexpected: {other:?}"),
        }
        let tokens = engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| {
                engine
                    .state()
                    .object(**id)
                    .is_some_and(|o| o.card.is_none() && o.kind == ObjectKind::Permanent)
            })
            .count();
        if tokens == 2 {
            break;
        }
        guard += 1;
        assert!(guard < 30, "expected 2 tokens, got {tokens}");
    }
}

#[test]
#[allow(clippy::too_many_lines)] // scenario script — step-by-step readability beats extraction
fn skyclave_exiles_and_owner_gets_illusion() {
    let mut engine = Engine::new(
        &preset(
            24,
            vec![skyclave(), plains(), plains(), plains()],
            vec![],
            vec![swords()],
            vec![ondu_cleric(), plains()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let cleric = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == ondu_cleric()))
        })
        .unwrap();

    // p0 casts the Apparition over three turns, then targets the cleric.
    let mut cast = false;
    let mut guard = 0;
    while !cast {
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
                        .is_some_and(|o| o.card.is_some_and(|d| d.index == skyclave()))
                }) {
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    cast = true;
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
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 100, "apparition never cast");
    }
    // Pass → ETB trigger → up-to-one target choice.
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
            } => {
                assert_eq!(player, p0);
                assert_eq!((min, max), (0, 1), "up-to-one target");
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
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 30, "no target choice appeared");
    }
    // Resolve the ETB ability (pass until the cleric is exiled).
    let mut guard = 0;
    while !engine
        .state()
        .zones
        .list(ZoneLocation::Exile(p1))
        .contains(&cleric)
    {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while resolving ETB: {other:?}"),
        }
        guard += 1;
        assert!(guard < 20, "cleric never exiled");
    }

    // p1 Swords the Apparition → LTB trigger → Illusion for the cleric's owner.
    let sk = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == skyclave()))
        })
        .unwrap();
    let mut guard = 0;
    let mut swords_cast = false;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p1 && !swords_cast => {
                if !legal.mana_abilities.is_empty() {
                    let sources = legal.mana_abilities.clone();
                    for source in sources {
                        engine
                            .apply(player, PlayerAction::ActivateManaAbility { source })
                            .unwrap();
                    }
                    continue;
                }
                let stp = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(p1))
                    .iter()
                    .copied()
                    .find(|id| {
                        engine
                            .state()
                            .object(*id)
                            .is_some_and(|o| o.card.is_some_and(|c| c.index == swords()))
                    })
                    .expect("swords still in hand");
                engine
                    .apply(player, PlayerAction::CastSpell { card: stp })
                    .unwrap();
                swords_cast = true;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert!(options.contains(&sk));
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![sk] })
                    .unwrap();
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
            other => panic!("unexpected: {other:?}"),
        }
        // Illusion on p1's battlefield with X/X = cleric cmc (2)?
        let illusion = engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .find(|id| {
                engine.state().object(*id).is_some_and(|o| {
                    o.card.is_none() && o.controller == p1 && o.characteristics().power == Some(2)
                })
            });
        if illusion.is_some() {
            break;
        }
        guard += 1;
        assert!(guard < 60, "illusion never appeared");
    }
}
