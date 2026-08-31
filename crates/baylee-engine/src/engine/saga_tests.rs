//! Saga tests (CR 714): lore counters on ETB + after the draw step,
//! chapter triggers, sacrifice after the final chapter.

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
fn urzas_saga() -> CardIndex {
    card_index("4c6a0c30-b547-4eff-8ff4-0ca25803c076")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

fn preset(seed: u64, hand0: Vec<CardIndex>) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(island())).collect();
    let mk = |hand: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck.clone(),
        sideboard: vec![],
        starting_life: None,
        starting_hand: Some(hand.into_iter().map(entry).collect()),
        starting_battlefield: vec![],
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
        seats: vec![mk(hand0), mk(vec![])],
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

/// Passes/answers everything until p0 has priority with the saga in
/// `legal.lands`, then plays it.
fn drive_and_play_saga(engine: &mut Engine<RegistryLookup>, p0: PlayerId) {
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 400, "no land-play window");
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if let Some(&card) = legal.lands.iter().find(|id| {
                    engine.state().object(**id).unwrap().card.unwrap().index == urzas_saga()
                }) {
                    engine
                        .apply(player, PlayerAction::PlayLand { card })
                        .unwrap();
                    return;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
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
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<_> = engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(player))
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
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
}

/// Passes/answers everything for `steps` pending choices.
fn drive(engine: &mut Engine<RegistryLookup>, steps: usize) {
    for _ in 0..steps {
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
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<_> = engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(player))
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
            Pending::ChooseCards { player, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                    .unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
}

fn saga_object(engine: &Engine<RegistryLookup>) -> Option<ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == urzas_saga())
        })
}

#[test]
fn saga_ticks_through_chapters_and_sacrifices_after_final() {
    let mut engine = Engine::new(&preset(3, vec![urzas_saga()]), RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    drive_and_play_saga(&mut engine, p0);
    // ETB: lore counter 1 + chapter I on the stack.
    let saga = saga_object(&engine).expect("saga on the battlefield");
    assert_eq!(
        engine
            .state()
            .object(saga)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::CounterKind::Lore),
        1
    );
    // Let chapter I resolve, then drive through two more of p0's turns
    // (each draw step adds a lore counter; chapter III ends the saga).
    drive(&mut engine, 200);
    // After p0's draw step: lore counter 2; after the next: 3, then the
    // saga is sacrificed (counters >= final chapter after III resolves).
    let still_there = saga_object(&engine);
    let in_graveyard = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(p0))
        .iter()
        .any(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == urzas_saga())
        });
    assert!(
        still_there.is_none() && in_graveyard,
        "saga sacrificed after chapter III (battlefield: {:?}, graveyard: {in_graveyard})",
        still_there.is_some()
    );
}
