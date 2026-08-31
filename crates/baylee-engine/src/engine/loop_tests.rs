//! Endless loops: the house rule that a real one resolves once and is
//! then broken, and the promise that a merely enormous game is left alone.
//!
//! The acceptance pool has no two cards that form a mandatory loop, so the
//! loop is built here instead: a creature whose enters-the-battlefield
//! trigger blinks itself. It enters, the trigger goes on the stack,
//! everyone passes, it resolves, the creature is exiled and returns — and
//! its trigger goes on the stack again. Nobody is ever given a choice that
//! could stop it, which is exactly what makes it endless.

use super::*;
use crate::loops::LoopWatch;
use crate::state::CardLookup;
use baylee_cards_dsl::{
    AbilityDef, CardDef, Coverage, Effect, FaceDef, Filter, PlayerRel, StepKind, TargetReq,
    TargetSpec, Trigger,
};
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, LoopPolicy, PrintInfo,
    SeatController, SeatSpec,
};
use baylee_core::types::TypeSet;

/// Index of the synthetic card; deliberately past the real registry so the
/// pool lookup below can be a pure fallback.
const LOOPING: CardIndex = CardIndex::new(60_000);
/// Index of the equally synthetic filler card the libraries are made of.
const FILLER: CardIndex = CardIndex::new(60_001);

static SELF_ONLY: Filter = Filter::This;
static BLINK_SELF: &[Effect] = &[Effect::Blink {
    target: TargetSpec::ThisObject,
}];

static LOOPING_CARD: CardDef = CardDef {
    index: LOOPING,
    oracle_id: "test-looping",
    scryfall_id: "test-looping",
    faces: &[FaceDef {
        name: "Möbius Familiar",
        types: TypeSet::CREATURE,
        power: Some(1),
        toughness: Some(1),
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        // Starts the loop: a permanent placed on the battlefield by the
        // preset never fired an enters trigger, so something has to light
        // the fuse.
        AbilityDef::Triggered {
            trigger: Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            effects: BLINK_SELF,
            targets: Some(TargetReq::one(TargetSpec::ThisObject)),
            once_per_turn: false,
        },
        // Sustains it: the creature that comes back enters, and asks to be
        // blinked again.
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&SELF_ONLY),
            effects: BLINK_SELF,
            targets: Some(TargetReq::one(TargetSpec::ThisObject)),
            once_per_turn: false,
        },
    ],
    ..CardDef::DEFAULT
};

static FILLER_CARD: CardDef = CardDef {
    index: FILLER,
    oracle_id: "test-filler",
    scryfall_id: "test-filler",
    faces: &[FaceDef {
        name: "Blank",
        types: TypeSet::LAND,
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    ..CardDef::DEFAULT
};

struct TestPool;
impl CardLookup for TestPool {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        match index {
            LOOPING => Some(&LOOPING_CARD),
            FILLER => Some(&FILLER_CARD),
            other => baylee_cards::by_index(other),
        }
    }
}

const fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

/// A duel where seat 0 already has the looping creature in play.
fn looping_game(policy: LoopPolicy) -> Engine<TestPool> {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(FILLER)).collect();
    let seat = |battlefield: Vec<DeckEntry>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck.clone(),
        sideboard: vec![],
        starting_life: None,
        starting_hand: Some(vec![]),
        starting_battlefield: battlefield,
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        format: FormatId::Freeform,
        seed: 7,
        dev_mode: false,
        house_rules: HouseRules {
            loop_policy: policy,
            ..HouseRules::default()
        },
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat(vec![entry(LOOPING)]), seat(vec![])],
    };
    let mut engine = Engine::new(&preset, TestPool).expect("duel starts");
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    engine
}

/// Answers whatever is pending in the most passive way available, up to
/// `limit` answers. Returns the number of answers actually given.
fn play_passively<L: CardLookup>(engine: &mut Engine<L>, limit: u32) -> u32 {
    for answered in 0..limit {
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
            Pending::ChooseTargets {
                player,
                options,
                min,
                ..
            } => {
                let objects = options.iter().take(min as usize).copied().collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<ObjectId> = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(player))
                    .iter()
                    .take(count as usize)
                    .copied()
                    .collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: hand })
                    .unwrap();
            }
            _ => return answered,
        }
    }
    limit
}

/// How many answers a loop needs before the watch has enough samples to
/// confirm it: the warm-up, plus room for Brent to save a tortoise and
/// come back to it twice.
const ENOUGH_TO_CONFIRM: u32 = (LoopWatch::WATCH_AFTER + LoopWatch::SAMPLE_EVERY * 24) as u32;

/// The house rule the user asked for: a real endless loop happens, and
/// then stops happening. The game keeps going.
#[test]
fn a_real_endless_loop_resolves_and_is_then_broken() {
    let mut engine = looping_game(LoopPolicy::RunOnceThenBreak);
    play_passively(&mut engine, ENOUGH_TO_CONFIRM);
    assert!(
        engine.loops_broken() >= 1,
        "the loop was never recognised as one"
    );
    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "breaking a loop must not end the game"
    );
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|e| matches!(e.event, GameEvent::LoopDetected { broken: true, .. })),
        "the break is not in the journal"
    );
    // The creature the loop was blinking is still there — the house rule
    // stops the repetition, it does not remove the board.
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter_map(|id| engine.state().object(*id))
            .any(|o| o.card.is_some_and(|c| c.index == LOOPING)),
        "breaking the loop ate the permanent"
    );
    // And the game moved on: a turn began after the break, which is what
    // "break it and keep playing" has to mean. (The card starts the loop
    // again next upkeep and is broken again — that is the same house rule
    // applied twice, not a failure to apply it once.)
    let entries = engine.state().journal.entries();
    let first_break = entries
        .iter()
        .position(|e| matches!(e.event, GameEvent::LoopDetected { .. }))
        .expect("a loop was detected");
    assert!(
        entries[first_break..]
            .iter()
            .any(|e| matches!(e.event, GameEvent::TurnStarted { .. })),
        "the game never reached another turn after the loop was broken"
    );
}

/// With the Comprehensive Rules policy the same loop is a draw
/// (CR 104.4b) rather than something to be broken.
#[test]
fn the_comp_rules_policy_makes_the_same_loop_a_draw() {
    let mut engine = looping_game(LoopPolicy::CompRulesDraw);
    play_passively(&mut engine, ENOUGH_TO_CONFIRM);
    match engine.pending() {
        Pending::GameOver(result) => {
            assert_eq!(result.winner, None);
            assert_eq!(result.reason, EndReason::Draw);
        }
        other => panic!("expected a draw, got {other:?}"),
    }
    assert_eq!(engine.loops_broken(), 0, "nothing should have been broken");
}

/// A game with nothing looping in it must never be flagged, however long
/// it is played out. This is the ally-deck guarantee in miniature: the
/// situation keeps changing, so there is no repeat to find.
#[test]
fn an_ordinary_long_game_is_never_called_a_loop() {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(FILLER)).collect();
    let seat = || SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck.clone(),
        sideboard: vec![],
        starting_life: None,
        starting_hand: Some(vec![]),
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        format: FormatId::Freeform,
        seed: 11,
        dev_mode: false,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat(), seat()],
    };
    let mut engine = Engine::new(&preset, TestPool).expect("duel starts");
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    play_passively(&mut engine, ENOUGH_TO_CONFIRM);
    assert_eq!(
        engine.loops_broken(),
        0,
        "a game that merely ran long was called a loop"
    );
}

/// The signature the detector compares is blind to object identity: a
/// permanent that left and came back is the same *situation*, even though
/// it is a different object with a later timestamp. Without this, no
/// endless loop would ever be recognised.
#[test]
fn the_signature_ignores_identity_but_not_the_situation() {
    let mut engine = looping_game(LoopPolicy::RunOnceThenBreak);
    play_passively(&mut engine, 8);
    let before = engine.state().loop_signature();
    let snapshot_before = engine.state().snapshot_hash();

    // Let the loop blink the creature at least once.
    play_passively(&mut engine, 8);
    assert_ne!(
        engine.state().snapshot_hash(),
        snapshot_before,
        "the game did not move at all"
    );

    // Walk on until the situation comes back around; with the blink loop
    // running it must, within a handful of answers.
    let mut returned = false;
    for _ in 0..64 {
        if engine.state().loop_signature() == before {
            returned = true;
            break;
        }
        play_passively(&mut engine, 1);
    }
    assert!(
        returned,
        "a looping game never returned to an earlier situation"
    );
}
