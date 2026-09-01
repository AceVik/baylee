//! Shared card-test kit: one place that builds a two-seat duel, deals
//! the mulligans, and answers the small questions card tests ask
//! ("is X on the battlefield?", "what are its projected P/T?").
//!
//! The point is that a behavioral card test should be ~10 lines: put a
//! card somewhere, walk the game to the moment its rules text matters,
//! assert on the state. New card tests use this kit instead of copying
//! the preset/mulligan helpers into another `*_tests.rs`.

use super::*;
use crate::state::CardLookup;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};

/// Registry lookup backed by the compiled card pool.
pub struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

/// Registry index by Scryfall oracle id (panics with a clear message).
#[track_caller]
pub fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("registry contains the card")
        .index
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

/// A two-seat duel under construction.
pub struct Duel {
    seed: u64,
    /// Whether the seats get the harness' own dev capability.
    dev: bool,
    hand: [Vec<CardIndex>; 2],
    battlefield: [Vec<CardIndex>; 2],
    sideboard: [Vec<CardIndex>; 2],
    library_filler: CardIndex,
}

impl Duel {
    /// A duel with `library_filler` as the 60-card backing deck (any
    /// basic land keeps draws legal and uninteresting).
    #[must_use]
    pub fn new(seed: u64, library_filler: CardIndex) -> Self {
        Self {
            seed,
            dev: true,
            hand: [Vec::new(), Vec::new()],
            battlefield: [Vec::new(), Vec::new()],
            sideboard: [Vec::new(), Vec::new()],
            library_filler,
        }
    }

    /// Cards in a seat's opening hand.
    #[must_use]
    pub fn hand(mut self, seat: usize, cards: &[CardIndex]) -> Self {
        self.hand[seat] = cards.to_vec();
        self
    }

    /// Cards a seat starts with on the battlefield.
    #[must_use]
    pub fn battlefield(mut self, seat: usize, cards: &[CardIndex]) -> Self {
        self.battlefield[seat] = cards.to_vec();
        self
    }

    /// Cards a seat keeps outside the game (sideboard; wish targets).
    #[must_use]
    pub fn sideboard(mut self, seat: usize, cards: &[CardIndex]) -> Self {
        self.sideboard[seat] = cards.to_vec();
        self
    }

    /// Builds the duel the way a lobby would: no seat may touch the state.
    #[must_use]
    pub const fn without_capabilities(mut self) -> Self {
        self.dev = false;
        self
    }

    /// Builds the engine (both seats AI-controlled; tests drive pending
    /// choices directly).
    #[track_caller]
    pub fn start(self) -> Engine<RegistryLookup> {
        let deck: Vec<DeckEntry> = (0..60).map(|_| entry(self.library_filler)).collect();
        let mk = |seat: usize| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            // A test harness is a host that trusts itself: it sets boards up
            // directly, which is what the capability is for.
            capabilities: baylee_core::preset::SeatCapabilities {
                dev_commands: self.dev,
                see_hidden: false,
            },
            deck: deck.clone(),
            sideboard: self.sideboard[seat].iter().copied().map(entry).collect(),
            starting_life: None,
            starting_hand: Some(self.hand[seat].iter().copied().map(entry).collect()),
            starting_battlefield: self.battlefield[seat].iter().copied().map(entry).collect(),
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: self.seed,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            seats: vec![mk(0), mk(1)],
        };
        Engine::new(&preset, RegistryLookup).expect("duel starts")
    }
}

/// Keeps both opening hands.
#[track_caller]
pub fn keep_mulligans(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

/// Advances until `seat` holds priority in their first main phase.
#[track_caller]
pub fn reach_main_phase(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    for _ in 0..20 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
        {
            return;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("never reached {seat:?}'s main phase");
}

/// Passes priority (declaring empty attackers/blockers on the way) until
/// `pred` holds. This is the "let the game run" primitive: spells
/// resolve, triggers resolve, turns advance.
#[track_caller]
pub fn pass_until(
    engine: &mut Engine<RegistryLookup>,
    pred: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..100 {
        if pred(engine) {
            return;
        }
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
            other => panic!("unexpected while passing: {other:?}"),
        }
    }
    panic!("condition never reached");
}

/// Walks the game until a target choice offers `wanted`, answering whatever is
/// asked on the way, and returns the options finally offered.
///
/// A copy effect asks twice — once for the copying trigger's own target, once
/// for the copy's new targets — so a test that cares about the second choice
/// needs to get past the first without hard-coding the order they arrive in.
#[must_use]
pub fn options_offered_including(
    engine: &mut Engine<RegistryLookup>,
    wanted: baylee_core::ids::ObjectId,
) -> Vec<baylee_core::ids::ObjectId> {
    for _ in 0..100 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                if options.contains(&wanted) {
                    return options;
                }
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while waiting for a choice of {wanted:?}: {other:?}"),
        }
    }
    panic!("no target choice ever offered {wanted:?}");
}

/// Whether `card` sits on the battlefield under `seat`'s control.
#[must_use]
pub fn on_battlefield(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Option<baylee_core::ids::ObjectId> {
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
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
}

/// Projected power/toughness of a battlefield object.
#[must_use]
pub fn pt(engine: &Engine<RegistryLookup>, object: baylee_core::ids::ObjectId) -> (i16, i16) {
    let c = engine
        .state()
        .object(object)
        .expect("object exists")
        .characteristics();
    (c.power.unwrap_or(0), c.toughness.unwrap_or(0))
}
