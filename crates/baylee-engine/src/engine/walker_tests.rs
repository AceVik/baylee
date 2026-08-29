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
fn jace() -> CardIndex {
    card_index("7f77a84e-5a4b-4834-aefa-3cecc175ae8e")
}
fn teferi() -> CardIndex {
    card_index("ae7604bb-4818-45a3-960c-cf3d83f15964")
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
        deck: deck.clone(),
        starting_life: None,
        starting_hand: None,
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
    let Pending::ChooseCards { .. } = engine.pending().clone() else {
        panic!("expected scry prompt, got {:?}", engine.pending())
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
