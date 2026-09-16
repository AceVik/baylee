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
            // "You may …" on the way past. Yes, because that is what these
            // tests meant before the word was read at all, and because a
            // test whose *subject* is the optional clause answers it itself
            // and never gets here.
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
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

/// Whether `card` sits in `seat`'s graveyard.
///
/// The third of the trio, and the one a *negative* outcome is read off:
/// a countered spell and a resolved one differ only in which zone the card
/// ends up in, so a test that could not look in a graveyard could only
/// prove that nothing happened.
#[must_use]
pub fn in_graveyard(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Option<baylee_core::ids::ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(seat))
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

/// Taps everything `seat` can tap for mana, then casts `card` from their hand.
///
/// The two halves are one step because they are one decision in a test: a
/// spell is cast off an open board, and a test that tapped nothing would be
/// refused by `LegalActions` for a reason that has nothing to do with what it
/// was written to prove.
#[track_caller]
pub fn cast_from_hand(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) {
    let spell = in_hand(engine, seat, card).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    engine
        .apply(seat, PlayerAction::CastSpell { card: spell })
        .unwrap();
}

/// Puts the top `n` cards of `seat`'s library into their graveyard.
///
/// `SeatSpec` has fields for an opening hand and a starting battlefield and
/// none for a graveyard, so every card that reads one — Bojuka Bog, anything
/// that reanimates — would otherwise need a spell cast first just to put
/// something there. This is the harness' own dev capability doing what the
/// capability is for.
#[track_caller]
pub fn seed_graveyard(engine: &mut Engine<RegistryLookup>, seat: PlayerId, n: usize) {
    let state = engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up");
    let cards: Vec<baylee_core::ids::ObjectId> = state
        .zones
        .list(crate::zone::ZoneLocation::Library(seat))
        .iter()
        .rev()
        .take(n)
        .copied()
        .collect();
    assert_eq!(
        cards.len(),
        n,
        "the library is shorter than the seed asked for"
    );
    for card in cards {
        state
            .move_object(
                card,
                crate::zone::ZoneLocation::Graveyard(seat),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("the harness moves a card");
    }
    // The offer was computed when priority was granted, which was before
    // this. An ability that reads a graveyard is withheld while no graveyard
    // holds what it needs, so without this the seeding is invisible to the
    // very question it was done for.
    engine.refresh_offer();
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
    if let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    {
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

// ---------------------------------------------------------------------------
// Driving one activation, and the board it is driven on
//
// Promoted out of `offer_tests` when a second pool-wide sweep needed the same
// walk. The reason it is shared rather than copied is `answer_one`'s match:
// it is exhaustive on purpose, so that a new `Pending` variant is a decision
// somebody makes rather than a case a `_` arm inherits — and two copies of an
// exhaustive match is two places to make that decision, one of which will be
// forgotten.
// ---------------------------------------------------------------------------

/// One thing an object was offered, in the three lists an offer can live in.
///
/// A mana ability belongs here on the same footing as any other: CR 605.1
/// only says it skips the stack, and `mana_abilities` is an enumeration in
/// exactly the same sense — the client's mana planner sends straight off it.
#[derive(Clone, Copy, Debug)]
pub enum Deed {
    /// `LegalActions::mana_abilities` named this object.
    Mana,
    /// `LegalActions::abilities` named this object and this index.
    Ability(u32),
    /// `LegalActions::castable` named this card in hand.
    Cast,
}

impl Deed {
    #[must_use]
    pub fn action(self, source: ObjectId) -> PlayerAction {
        match self {
            Self::Mana => PlayerAction::ActivateManaAbility { source },
            Self::Ability(ability_index) => PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
            Self::Cast => PlayerAction::CastSpell { card: source },
        }
    }
}

/// Four of each registered basic: twenty mana of every colour, which is
/// enough that affordability never decides whether an ability is offered.
///
/// It has to be enough, because an ability the engine declines to offer for
/// want of mana is an ability a sweep over the pool never presses.
pub fn basics() -> Vec<CardIndex> {
    let mut field = Vec::new();
    for basic in baylee_cards::decks::basic_lands().into_iter().flatten() {
        field.extend(std::iter::repeat_n(basic, 4));
    }
    field
}

/// The quietest creature in the pool.
///
/// Twenty basics cannot supply two things a probe board wants: a creature to
/// sacrifice or to point at, and a creature *card* in a graveyard to bring
/// back. One card serves both, and Llanowar Elves is the one that brings
/// least of its own — its whole text is one mana ability, so no trigger
/// fires, no static applies and no keyword changes what anything else on the
/// board can do.
///
/// The pool has no vanilla creature at all: the only implemented creatures
/// with an empty `abilities` list are the five werewolves, and daybound
/// would put every probe board into a *game state* (CR 731) the card under
/// test can read. One mana ability nothing presses is the smaller footprint
/// — and nothing presses it, because [`deeds`] only ever looks at the
/// probed card's own objects.
pub fn quiet_creature() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// The quietest artifact in the pool, for the same job one type up.
///
/// Twenty basics and a handful of Elves answer "target creature", "target
/// land" and "target permanent", and answer nothing at all to "target
/// artifact" — so every card in the pool that points at one was going
/// unpressed. Sol Ring is Llanowar Elves again in artifact form: its whole
/// printed text is one mana ability, so no trigger fires, no static applies
/// and nothing it does changes what another permanent is.
///
/// There is no enchantment beside it, and that is a fact about the pool
/// rather than an omission. Of the implemented enchantments, every single
/// one prints a static, a trigger or a replacement effect — the quietest are
/// Rhystic Study and Smothering Tithe, which watch what the *opponent* does
/// — so putting any of them on a probe board would put a rule on it. The
/// card that would do the job does not exist yet; when one is adopted this
/// is where it goes.
pub fn quiet_artifact() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

/// The seed [`arena`] runs at.
///
/// Fixed rather than swept, for the reason `offer_tests` gives: the board is
/// built by hand down to the last permanent, so all a varying seed would
/// move is which seat takes the first turn.
pub const SEED: u64 = 4_211;

/// Whether a card's front face is something that can sit on a battlefield.
///
/// The same five types `decks::probe_preset` asks about, and for the same
/// reason: `starting_battlefield` puts a card there without casting it, so
/// handing it an instant would be building a board no game could reach. A
/// card that fails this is still probed — in hand, where cycling and its
/// relatives live (`ActivationZone::Hand`).
pub fn is_permanent(def: &baylee_cards_dsl::CardDef) -> bool {
    def.faces.first().is_some_and(|f| {
        f.types.contains(TypeSet::LAND)
            || f.types.contains(TypeSet::CREATURE)
            || f.types.contains(TypeSet::ARTIFACT)
            || f.types.contains(TypeSet::ENCHANTMENT)
            || f.types.contains(TypeSet::PLANESWALKER)
    })
}

/// Answers mulligans, priorities and empty combat declarations until `seat`
/// holds priority in its own first main phase.
///
/// Tolerant where [`reach_main_phase`] panics: this walker is *setup* for
/// hundreds of unrelated cards, and a board that asks a question it does not
/// know how to answer on the way is a card the caller cannot probe rather
/// than a failure. The distinction is kept honest by a *counted floor* in
/// whichever sweep calls this, which notices when "cannot probe" starts
/// meaning "most of the pool". Every caller owes one.
///
/// Three of those questions are asked by a permanent *on the way in*, and
/// the walker answers them rather than abandoning the board — twenty-three
/// implemented cards, the ten shocklands among them, had no probe at all
/// until it did. What it must not do is answer them the way
/// [`drive_to_rest`] does: the driver is exercising an effect and takes an
/// option wherever one is offered, and this is setup, which wants the card
/// the pool prints. So an optional choice is **declined** — exactly `min`
/// targets, which is none of them for the copy clause on Phantasmal Image
/// and its relatives, because a Cursed Mirror that entered as a copy of
/// something else is not the card the caller came to probe.
///
/// The shockland's question is the one that has no "decline": both answers
/// are legal and one of them puts the land onto the battlefield tapped,
/// where it offers nothing and the probe is pointless. It is answered yes,
/// which is also the answer that leaves the land in the state every other
/// land on this board is already in.
pub fn walk_to_own_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> bool {
    for _ in 0..60 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return true;
        }
        let refused = match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).is_err()
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).is_err()
            }
            Pending::ChooseAttackers { player, .. } => engine
                .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                .is_err(),
            Pending::ChooseBlockers { player, .. } => engine
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .is_err(),
            Pending::ChooseTargets {
                player,
                options,
                min,
                ..
            } => engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: options.into_iter().take(usize::from(min)).collect(),
                        players: Vec::new(),
                    },
                )
                .is_err(),
            Pending::ChooseSubtype { player, options } => match options.first().copied() {
                Some(first) => engine
                    .apply(player, PlayerAction::ChooseSubtype(first))
                    .is_err(),
                None => return false,
            },
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { .. },
                ..
            } => engine.apply(player, PlayerAction::YesNo(true)).is_err(),
            _ => return false,
        };
        if refused {
            return false;
        }
    }
    false
}

/// Every *activation* `legal` offers on `objects`, each named by its
/// *position* in that slice rather than by its `ObjectId`, so the answer
/// survives being carried to a freshly built board.
///
/// Casting is deliberately not here. A sweep that asks whether every offered
/// activation can actually be performed is asking about `abilities` and
/// `mana_abilities`; `castable` is a third list with its own sweep, and
/// folding it in would silently move that sweep's counted floors. See
/// [`presses`] for the other reading.
pub fn deeds(legal: &LegalActions, objects: &[ObjectId]) -> Vec<(usize, Deed)> {
    let mut found = Vec::new();
    for (slot, object) in objects.iter().enumerate() {
        if legal.mana_abilities.contains(object) {
            found.push((slot, Deed::Mana));
        }
        found.extend(
            legal
                .abilities
                .iter()
                .filter(|(source, _)| source == object)
                .map(|(_, index)| (slot, Deed::Ability(*index))),
        );
    }
    found
}

/// Everything on `objects` that could make the engine *do* something to the
/// board, named by position for [`deeds`]' reason.
///
/// The other half of the same offer, and the two differ by one variant each
/// because they are asked for different reasons. A mana ability is left out:
/// it targets nothing, and pressing it would tap the very source an ability
/// with a `{T}` cost still needs. Casting is put in: a spell is most of what
/// the pool prints, and a sweep that only ever pressed abilities would never
/// see an instant at all.
pub fn presses(legal: &LegalActions, objects: &[ObjectId]) -> Vec<(usize, Deed)> {
    let mut found = Vec::new();
    for (slot, object) in objects.iter().enumerate() {
        if legal.castable.contains(object) {
            found.push((slot, Deed::Cast));
        }
        found.extend(
            legal
                .abilities
                .iter()
                .filter(|(source, _)| source == object)
                .map(|(_, index)| (slot, Deed::Ability(*index))),
        );
    }
    found
}
/// Where driving one activation to a rest ended.
///
/// Every answer the driver gives is one the question itself enumerated, so
/// [`Rest::Refused`] is the offer contradiction one question further in: the
/// engine listed an answer and then rejected it.
#[derive(Debug)]
pub enum Rest {
    /// Back to a quiet priority: the stack is empty and the seat holds it.
    Reached,
    /// The game ended on the way, which is a rest of its own.
    Over,
    /// A pending this driver has no arm for. Not a failure — the driver is
    /// deliberately not a second house AI — but counted, because "cannot
    /// answer this one" growing into "cannot answer anything" is how a sweep
    /// dies quietly.
    Unanswered(&'static str),
    /// The step cap ran out.
    Stalled,
    /// An answer taken from the enumeration was refused.
    Refused(String),
}

/// The one question this driver answers, taken out of what the question
/// itself enumerated.
///
/// Split out of [`drive_to_rest`] so a sweep can *intercept* one kind of
/// question and delegate the rest. Target locality is the case that needed
/// it: it has to answer `ChooseTargets` itself — it is the choice under test
/// — while every other question is answered here, by the same arms the offer
/// sweep uses.
///
/// `Err` is a rest reached on the way: the game ended, or the question has no
/// arm. Nothing is applied — the caller applies what comes back, which is
/// also what lets it read the answer it gave.
///
/// Every answer is taken out of what the question itself enumerated. That is
/// the offer invariant carried past `LegalActions`: `ChooseTargets` carries
/// the legal targets, `ChooseCards` the legal cards, `ChooseColor` the legal
/// colours. Each is a promise of the same kind, and an answer lifted straight
/// out of one and then refused is the same two-probes disagreement.
///
/// The match is exhaustive on purpose. A new `Pending` variant is a new
/// question the engine can ask, and the choice of whether a sweep answers it
/// or counts it as unreached should be made once, here, when it is added —
/// not inherited from a `_` arm in each of the sweeps that share this.
///
/// The two departures are `DiscardChoice` and `MulliganBottom`, which
/// enumerate a *count* and leave the cards to the seat's own hand — so the
/// hand is where those answers come from, exactly as a client reads them.
#[allow(clippy::too_many_lines)] // one flat arm per question the engine asks
pub fn answer_one(engine: &Engine<RegistryLookup>) -> Result<(PlayerId, PlayerAction), Rest> {
    Ok(match engine.pending().clone() {
        Pending::GameOver(_) => return Err(Rest::Over),
        Pending::Mulligan { .. } => return Err(Rest::Unanswered("Mulligan")),
        Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
        Pending::ChooseAttackers { player, .. } => {
            (player, PlayerAction::DeclareAttackers { attackers: vec![] })
        }
        Pending::ChooseBlockers { player, .. } => {
            (player, PlayerAction::DeclareBlockers { blockers: vec![] })
        }
        Pending::ChooseTargets {
            player,
            options,
            player_options,
            min,
            max,
            ..
        } => {
            // An "up to" prompt (`min` 0) is answered with one anyway:
            // choosing nothing is legal and exercises nothing.
            let want = usize::from(min).max(1).min(usize::from(max));
            let objects: Vec<_> = options.into_iter().take(want).collect();
            let players = player_options
                .into_iter()
                .take(want.saturating_sub(objects.len()))
                .collect();
            (player, PlayerAction::ChooseTargets { objects, players })
        }
        Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } => {
            let want = usize::from(min).max(1).min(usize::from(max));
            (
                player,
                PlayerAction::ChooseObjects {
                    objects: options.into_iter().take(want).collect(),
                },
            )
        }
        Pending::LegendChoice { player, options } => (
            player,
            PlayerAction::ChooseObjects {
                objects: options.into_iter().take(1).collect(),
            },
        ),
        Pending::DiscardChoice { player, count } | Pending::MulliganBottom { player, count } => {
            let hand = engine
                .state()
                .zones
                .list(crate::zone::ZoneLocation::Hand(player))
                .clone();
            (
                player,
                PlayerAction::ChooseObjects {
                    objects: hand.into_iter().take(usize::from(count)).collect(),
                },
            )
        }
        Pending::ChooseSubtype { player, options } => {
            let Some(first) = options.first().copied() else {
                return Err(Rest::Unanswered("ChooseSubtype"));
            };
            (player, PlayerAction::ChooseSubtype(first))
        }
        Pending::ChooseColor { player, options } => {
            let Some(first) = options.first().copied() else {
                return Err(Rest::Unanswered("ChooseColor"));
            };
            (player, PlayerAction::ChooseColor(first))
        }
        Pending::ChoosePlayer { player, options } => {
            let Some(first) = options.first().copied() else {
                return Err(Rest::Unanswered("ChoosePlayer"));
            };
            (player, PlayerAction::ChoosePlayer(first))
        }
        Pending::ChooseCastMode {
            player, options, ..
        } => {
            let Some(first) = options.first() else {
                return Err(Rest::Unanswered("ChooseCastMode"));
            };
            (player, PlayerAction::ChooseMode(first.index as usize))
        }
        Pending::ChooseNumber { player, min, .. } => (player, PlayerAction::ChooseNumber(min)),
        Pending::YesNo { player, .. } => (player, PlayerAction::YesNo(true)),
        Pending::OrderObjects { player, objects } => {
            (player, PlayerAction::OrderObjects { objects })
        }
    })
}

/// Whether `seat` is back at a quiet priority: the stack is empty and the
/// seat holds it.
#[must_use]
pub fn at_rest(engine: &Engine<RegistryLookup>, seat: PlayerId) -> bool {
    engine.state().zones.stack_is_empty()
        && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
}

/// Answers whatever the engine asks, through [`answer_one`], until the
/// activation has resolved and `seat` is back at a quiet priority.
pub fn drive_to_rest(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> Rest {
    for _ in 0..200 {
        if at_rest(engine, seat) {
            return Rest::Reached;
        }
        let (player, action) = match answer_one(engine) {
            Ok(pair) => pair,
            Err(rest) => return rest,
        };
        if let Err(err) = engine.apply(player, action.clone()) {
            return Rest::Refused(format!(
                "{action:?} came out of the question, then: {err:?}"
            ));
        }
    }
    Rest::Stalled
}

/// A board where every filter this sweep can afford has **two** of something.
///
/// Seat 0 gets the card (on the battlefield too, when it is a permanent),
/// twenty basics for mana, three [`quiet_creature`]s with two more of them
/// put into the graveyard before anything is asked, and two
/// [`quiet_artifact`]s; seat 1 gets two Elves and one artifact. That is a
/// second creature for "target creature", a second creature an opponent
/// controls, a second creature card in a graveyard, a second artifact and an
/// artifact an opponent controls, and — from the basics — a second land and a
/// second permanent, which between them are most of the target specs the pool
/// prints.
///
/// The Elves, the Sol Rings and the card's own permanent are the only things
/// left untapped: an ability whose cost is `{T}` must not have spent its
/// source paying for the mana that pays for it, and a bystander that tapped
/// for mana is a bystander whose status has already changed for a reason of
/// its own.
///
/// Returns the engine and the card's objects, battlefield first then hand —
/// a fixed order, so a press found on one board addresses the same object on
/// the next.
pub fn arena(card: CardIndex) -> Option<(Engine<RegistryLookup>, Vec<ObjectId>)> {
    let seat = PlayerId::new(0);
    let def = baylee_cards::by_index(card)?;
    let elf = quiet_creature();
    let rock = quiet_artifact();
    let mut field = basics();
    if is_permanent(def) {
        field.insert(0, card);
    }
    field.extend([elf; 5]);
    field.extend([rock; 2]);
    let filler = baylee_cards::decks::basic_lands()
        .into_iter()
        .flatten()
        .next()
        .unwrap_or(card);
    let mut engine = Duel::new(SEED, filler)
        .battlefield(0, &field)
        .battlefield(1, &[elf, elf, rock])
        .hand(0, &[card])
        .start();
    // Two of seat 0's five Elves into the graveyard, from the *end* of the
    // battlefield list: the card under test may itself be Llanowar Elves, and
    // the probed permanent has to be one of the ones that survives.
    for _ in 0..2 {
        let doomed = *engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .rfind(|id| {
                engine
                    .state()
                    .object(**id)
                    .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == elf))
            })?;
        sba::destroy(engine.dev_state_mut(seat)?, doomed);
    }
    if !walk_to_own_main(&mut engine, seat) {
        return None;
    }
    let objects: Vec<ObjectId> = mine(&engine, seat, card, Zone::Battlefield)
        .into_iter()
        .chain(mine(&engine, seat, card, Zone::Hand))
        .collect();
    if objects.is_empty() {
        return None;
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        return None;
    };
    for source in legal.mana_abilities {
        let is_fodder = engine.state().object(source).is_some_and(|o| {
            o.card
                .is_some_and(|c| c.index == card || c.index == elf || c.index == rock)
        });
        if is_fodder {
            continue;
        }
        // A mana ability offered and then refused is the offer sweep's
        // finding, not this one's: the board is abandoned rather than
        // unwrapped through.
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .ok()?;
    }
    Some((engine, objects))
}

/// Every object `seat` has in `zone` that came from `card`.
pub fn mine(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    zone: Zone,
) -> Vec<ObjectId> {
    let location = match zone {
        Zone::Battlefield => ZoneLocation::Battlefield,
        Zone::Hand => ZoneLocation::Hand(seat),
        Zone::Graveyard => ZoneLocation::Graveyard(seat),
        _ => return Vec::new(),
    };
    engine
        .state()
        .zones
        .list(location)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}
