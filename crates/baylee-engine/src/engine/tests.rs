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

fn creature() -> CardIndex {
    // Ondu Cleric — a 1/1 with no S2-relevant abilities.
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}

fn preset_2p(seed: u64, deck: &[CardIndex]) -> GamePreset {
    let entries: Vec<DeckEntry> = deck
        .iter()
        .cycle()
        .take(60)
        .map(|c| DeckEntry {
            card: *c,
            print: PrintRef::new(0),
        })
        .collect();
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
        seats: (0..2)
            .map(|_| SeatSpec {
                controller: SeatController::Ai(AIProfile::default()),
                capabilities: baylee_core::preset::SeatCapabilities::default(),
                deck: entries.clone(),
                sideboard: vec![],
                starting_life: None,
                starting_hand: None,
                starting_battlefield: vec![],
                emblems: vec![],
                team: None,
            })
            .collect(),
    }
}

fn keep_all(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            Pending::MulliganBottom { player, .. } => {
                let hand: Vec<_> = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..1].to_vec(),
                        },
                    )
                    .unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

/// The layer refresh reads `Zones::stack_projectable` instead of walking the
/// stack, so the two are only allowed to disagree about abilities.
///
/// Both directions are silent failures if they ever drift. An id left in
/// the subset after its object is gone gets projected forever; a spell
/// missing from it simply stops being affected by anthems, and nothing
/// anywhere would report either.
fn check_projectable(engine: &Engine<RegistryLookup>) {
    let state = engine.state();
    let expected: Vec<_> = state
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|o| o.kind != crate::object::ObjectKind::AbilityOnStack)
        })
        .collect();
    let mut actual = state.zones.stack_projectable().to_vec();
    let mut sorted = expected.clone();
    actual.sort_unstable();
    sorted.sort_unstable();
    assert_eq!(
        actual,
        sorted,
        "the projectable subset drifted from the stack (stack: {:?})",
        state.zones.list(ZoneLocation::Stack)
    );
}

fn pass_all(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..4 {
        check_projectable(engine);
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
            _ => return,
        }
    }
}

#[test]
fn full_turn_cycle_works() {
    let mut engine = Engine::new(&preset_2p(42, &[forest()]), RegistryLookup).unwrap();
    keep_all(&mut engine);
    // Turn 1: untap → upkeep → draw (skipped for P1) → main.
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if player.get() == 0));
    pass_all(&mut engine);
    // After enough passing, turn 2 begins for player 2.
    let mut guard = 0;
    while !(engine.state().turn.number == 2 && engine.state().turn.active.get() == 1) {
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
            other => panic!("unexpected pending: {other:?}"),
        }
        guard += 1;
        assert!(guard < 50, "no progress: {:?}", engine.pending());
    }
    assert_eq!(engine.state().turn.number, 2);
}

#[test]
fn determinism_through_engine() {
    let mut a = Engine::new(&preset_2p(7, &[forest()]), RegistryLookup).unwrap();
    let mut b = Engine::new(&preset_2p(7, &[forest()]), RegistryLookup).unwrap();
    keep_all(&mut a);
    keep_all(&mut b);
    assert_eq!(a.snapshot_hash(), b.snapshot_hash());
    for _ in 0..20 {
        let pending = a.pending().clone();
        let Pending::Priority { player, .. } = pending else {
            break;
        };
        a.apply(player, PlayerAction::PassPriority).unwrap();
        b.apply(player, PlayerAction::PassPriority).unwrap();
        assert_eq!(a.snapshot_hash(), b.snapshot_hash());
    }
}

#[test]
fn land_play_and_mana_and_cast() {
    // Deck with forests + cheap creatures; scripted to find them.
    let mut engine = Engine::new(&preset_2p(11, &[forest(), creature()]), RegistryLookup).unwrap();
    keep_all(&mut engine);
    // Player 1 main phase: play a land if possible, else pass.
    let p0 = PlayerId::new(0);
    let mut played_land = false;
    let mut cast_creature = false;
    for _ in 0..200 {
        let pending = engine.pending().clone();
        match pending {
            Pending::Priority { player, legal } => {
                if player == p0 && !played_land && !legal.lands.is_empty() {
                    engine
                        .apply(
                            player,
                            PlayerAction::PlayLand {
                                card: legal.lands[0],
                            },
                        )
                        .unwrap();
                    played_land = true;
                } else if player == p0
                    && played_land
                    && !legal.mana_abilities.is_empty()
                    && !cast_creature
                {
                    engine
                        .apply(
                            player,
                            PlayerAction::ActivateManaAbility {
                                source: legal.mana_abilities[0],
                            },
                        )
                        .unwrap();
                } else if player == p0
                    && played_land
                    && !legal.castable.is_empty()
                    && !cast_creature
                {
                    let card = legal.castable[0];
                    let is_creature = engine
                        .state()
                        .object(card)
                        .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE));
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    if is_creature {
                        cast_creature = true;
                    }
                } else {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
            }
            Pending::GameOver(_) => break,
            _ => {
                // combat declarations: declare nothing
                break;
            }
        }
        if cast_creature {
            break;
        }
    }
    assert!(
        played_land,
        "expected to play a land; pending: {:?}",
        engine.pending()
    );
}

#[test]
fn combat_kills_and_wins() {
    // Player 0 starts with 20 creatures on the battlefield via preset.
    let mut preset = preset_2p(3, &[forest()]);
    preset.seats[0].starting_battlefield = (0..20)
        .map(|_| DeckEntry {
            card: creature(),
            print: PrintRef::new(0),
        })
        .collect();
    preset.seats[1].starting_life = Some(5);
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    keep_all(&mut engine);
    // Walk to combat and attack with everything.
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut guard = 0;
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                let attackers: Vec<(ObjectId, baylee_core::ids::Defender)> = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .copied()
                    .filter(|id| combat::can_attack(engine.state(), player, *id))
                    .map(|id| (id, baylee_core::ids::Defender::Player(p1)))
                    .collect();
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::GameOver(result) => {
                assert_eq!(result.winner, Some(crate::win::Victor::Player(p0)));
                return;
            }
            other => panic!("unexpected pending: {other:?}"),
        }
        guard += 1;
        assert!(guard < 200, "game did not end");
    }
}
