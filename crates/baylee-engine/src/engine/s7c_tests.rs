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

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn phantasmal_image() -> CardIndex {
    card_index("bde94af8-faea-41ff-8eed-ba642eac9968")
}
fn rite_of_replication() -> CardIndex {
    card_index("fb60739e-1dc3-481d-a056-ad72e665c680")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

fn preset(seed: u64, hand0: Vec<CardIndex>, bf0: Vec<CardIndex>) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(island())).collect();
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
        seats: vec![mk(hand0, bf0), mk(vec![], vec![])],
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
fn clone_enters_as_copy_of_target() {
    let mut engine = Engine::new(
        &preset(
            61,
            vec![phantasmal_image(), island(), island()],
            vec![ondu_cleric()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let cleric = engine.state().zones.list(ZoneLocation::Battlefield)[0];

    // Walk to p0's main phase (mana expires at step end).
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

    // Play island, cast the image.
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
                } else if let Some(&card) = legal.castable.iter().find(|c| {
                    engine
                        .state()
                        .object(**c)
                        .is_some_and(|o| o.card.is_some_and(|d| d.index == phantasmal_image()))
                }) {
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    break;
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
        assert!(guard < 200);
    }
    // Resolve the spell → copy choice as it enters.
    let p1 = PlayerId::new(1);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!()
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!()
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected copy choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&cleric));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();

    // The image is now a cleric (copiable base replaced).
    let _ = p1;
    let image = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == phantasmal_image()))
        })
        .unwrap();
    let c = engine.state().object(image).unwrap().characteristics();
    assert_eq!(c.power, Some(1));
    assert_eq!(c.toughness, Some(1));
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::ALLY)
    );
}

#[test]
fn kicked_rite_makes_five_tokens() {
    let mut engine = Engine::new(
        &preset(62, vec![rite_of_replication()], {
            let mut bf = vec![ondu_cleric()];
            bf.extend([island(); 9]);
            bf
        }),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let cleric = engine.state().zones.list(ZoneLocation::Battlefield)[0];

    // Walk to p0's main phase (mana expires at step end).
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

    // Cast with kicker (9 mana from islands — need 9: {2}{U}{U}+{5}).
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
                let rite = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(p0))
                    .iter()
                    .copied()
                    .find(|id| {
                        engine.state().object(*id).is_some_and(|o| {
                            o.card.is_some_and(|c| c.index == rite_of_replication())
                        })
                    })
                    .unwrap();
                engine
                    .apply(player, PlayerAction::CastSpell { card: rite })
                    .unwrap();
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 40, "rite never cast");
    }
    // Target the cleric, then answer the kicker yes/no with yes.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected targets, got {:?}", engine.pending())
    };
    assert!(options.contains(&cleric));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    let Pending::YesNo { .. } = engine.pending().clone() else {
        panic!("expected kicker choice, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    // Resolve: 5 cleric tokens.
    let p1 = PlayerId::new(1);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!()
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!()
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    let tokens = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_none())
        })
        .count();
    assert_eq!(tokens, 5, "kicked rite should make five tokens");
    let _ = p1;
}
