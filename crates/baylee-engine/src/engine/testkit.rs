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
    commanders: [Vec<CardIndex>; 2],
    life: [Option<i32>; 2],
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
            commanders: [Vec::new(), Vec::new()],
            life: [None, None],
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

    /// A seat's starting life, overriding the format's.
    ///
    /// What it is for is separating a loss from the life total that usually
    /// comes with it: commander damage (CR 903.10a) kills a player who is
    /// nowhere near dying, and a test run at twenty life would watch them
    /// lose and be unable to say which rule did it.
    #[must_use]
    pub fn life(mut self, seat: usize, amount: i32) -> Self {
        self.life[seat] = Some(amount);
        self
    }

    /// Cards a seat keeps outside the game (sideboard; wish targets).
    #[must_use]
    pub fn sideboard(mut self, seat: usize, cards: &[CardIndex]) -> Self {
        self.sideboard[seat] = cards.to_vec();
        self
    }

    /// A seat's commanders, which start in the command zone (CR 903.6).
    ///
    /// The format stays `Freeform`: the seat lists the commander, so the
    /// commander rules that key off that list run, and the test does not
    /// silently inherit 40 life for a duel it is counting damage in.
    #[must_use]
    pub fn commander(mut self, seat: usize, cards: &[CardIndex]) -> Self {
        self.commanders[seat] = cards.to_vec();
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
            commanders: self.commanders[seat].iter().copied().map(entry).collect(),
            starting_life: self.life[seat],
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

/// Whether `card` sits in `seat`'s hand.
///
/// By index rather than by position: a seat's opening hand is the cards the
/// test named *plus* seven draws off the filler deck, so `list(Hand)[0]` is
/// only the seeded card by luck of the ordering.
#[must_use]
pub fn in_hand(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Option<baylee_core::ids::ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(seat))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// Taps everything that makes mana for `seat` except `keep`.
///
/// The exception is the point: a land whose *other* ability the test is
/// about to activate (Riptide Laboratory taps for {C} and also returns a
/// Wizard) would otherwise be spent paying for itself.
#[track_caller]
pub fn tap_mana_except(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    keep: baylee_core::ids::ObjectId,
) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        if source == keep {
            continue;
        }
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
}

/// Advances until `seat` holds priority in their first main phase, however
/// many turns away that is.
///
/// [`reach_main_phase`] answers `Pending::Priority` and nothing else, so it
/// cannot cross a turn boundary: the combat phase on the way asks for
/// attackers and it panics with `expected priority, got ChooseAttackers`.
/// This is [`pass_until`] with that predicate spelled once, for the tests
/// whose question belongs to the *other* seat's turn.
#[track_caller]
pub fn reach_their_main_phase(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    pass_until(engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == seat
    });
}

/// Whether the stack has nothing on it.
///
/// The end of a spell is not the end of what it started: a permanent
/// entering puts its triggers on the stack, so a test that read the board
/// the moment the creature arrived would count the counters before they
/// were placed.
#[must_use]
pub fn stack_is_empty(engine: &Engine<RegistryLookup>) -> bool {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .is_empty()
}

/// A basic Forest, the filler every pool-wide sweep builds its deck from.
///
/// It prints no `enter_modifiers`, one basic land type and one mana ability,
/// so a sweep can never mistake its own filler for the card under test.
#[must_use]
pub fn basic_forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// Plays `card` as a land on turn one, taking face `face` when the engine
/// offers the choice, and hands back the game with the permanent in it.
///
/// This is the board every land sweep starts from, and it has to be a real
/// `PlayLand`. `SeatSpec::starting_battlefield` seeds a permanent with
/// `move_object(.., Cause::Setup)`, which is a placement rather than an
/// entry: no replacement effect looks at it, so a board built that way
/// arrives untapped whatever the card says. A sweep resting on it would
/// measure nothing and pass.
///
/// # Errors
/// Anything that stopped the card reaching the battlefield as `face`, spelled
/// as prose a sweep can print beside the card's name. A land in an opening
/// hand, on an empty board, in a first main phase has nothing standing
/// between it and play, so every one of these is a finding rather than a
/// reason to skip.
pub fn play_land_face(
    card: CardIndex,
    face: usize,
) -> Result<(Engine<RegistryLookup>, baylee_core::ids::ObjectId), String> {
    let seat = PlayerId::new(0);
    let mut engine = Duel::new(7, basic_forest()).hand(0, &[card]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);

    let object = in_hand(&engine, seat, card).ok_or("never reached the hand")?;
    engine
        .apply(seat, PlayerAction::PlayLand { card: object })
        .map_err(|err| format!("was refused the land drop: {err:?}"))?;

    // A card printing two land faces is asked which one is being played, and
    // the answer decides which face's modifiers and abilities are the ones
    // in play.
    if let Pending::ChooseCastMode { player, options } = engine.pending().clone() {
        let slot = options
            .iter()
            .position(
                |o| matches!(o.kind, crate::choice::CastModeKind::PlayLandFace(f) if f == face),
            )
            .ok_or_else(|| format!("was never offered its own land face {face}"))?;
        engine
            .apply(player, PlayerAction::ChooseMode(slot))
            .map_err(|err| format!("refused the face choice: {err:?}"))?;
    }

    let landed = engine
        .state()
        .object(object)
        .ok_or("was played and then vanished")?;
    if landed.zone != crate::zone::Zone::Battlefield {
        return Err(format!("was played and is in {:?}", landed.zone));
    }
    if usize::from(landed.face_index) != face {
        return Err(format!(
            "was played as face {} when face {face} was asked for",
            landed.face_index
        ));
    }
    Ok((engine, object))
}
