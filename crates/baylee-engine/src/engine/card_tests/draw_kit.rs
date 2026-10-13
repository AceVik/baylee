//! Shared pieces of the draw-replacement card tests (K12 of the partials
//! lane): Golgari Thug, Dakmor Salvage, Chains of Mephistopheles and
//! Aladdin's Lamp.
//!
//! Nothing here knows what a card is supposed to do. It names the cards the
//! four tests share and the plumbing around a draw: the cards that make one
//! (Jayemdae Tome for a single draw, Ancestral Recall for three, Howling Mine
//! for the second of a draw step, Phyrexian Arena for one in the upkeep), the
//! reader of the question the engine asks about a draw, and the walker that
//! plays on until it is asked.
//!
//! **Why the walkers are not `pass_until`.** `pass_until` panics on a
//! `Pending::ChooseCards`, which is exactly what a draw with a dredge card in
//! a graveyard turns into. These stop on it and say so.

#[allow(clippy::wildcard_imports)] // the card tests' own vocabulary
use super::*;
use crate::turn::{Phase, Step};

pub(super) const P0: PlayerId = PlayerId::new(0);
pub(super) const P1: PlayerId = PlayerId::new(1);

/// Golgari Thug: {1}{B} 1/1, "When this creature dies, put target creature
/// card from your graveyard on top of your library." and Dredge 4.
pub(super) fn thug() -> CardIndex {
    card_index("a426a258-fd8b-489c-8642-9868ee47de85")
}

/// Dakmor Salvage: a land that enters tapped, taps for {B}, and has Dredge 2.
pub(super) fn salvage() -> CardIndex {
    card_index("cdc4048a-73ec-4ec1-a179-2b36c397bf1a")
}

/// Chains of Mephistopheles: {1}{B} enchantment.
pub(super) fn chains() -> CardIndex {
    card_index("eae87919-6322-4bd2-ae9c-b1ce25d686da")
}

/// Aladdin's Lamp: {10} artifact, "{X}, {T}: the next time you would draw a
/// card this turn, instead look at the top X cards ..."
pub(super) fn lamp() -> CardIndex {
    card_index("2aa2f96b-5784-4767-b9ea-b8d9222cb1de")
}

/// Ancestral Recall: "Target player draws three cards."
pub(super) fn recall() -> CardIndex {
    card_index("550c74d4-1fcb-406a-b02a-639a760a4380")
}

/// Jayemdae Tome: "{4}, {T}: Draw a card."
pub(super) fn tome() -> CardIndex {
    card_index("39ee576a-0803-4063-9c84-f2b537e4d44c")
}

/// Howling Mine: each player's draw step, that player draws an additional
/// card.
pub(super) fn mine() -> CardIndex {
    card_index("d26b27db-a567-4631-b4b6-7294222fbdd1")
}

/// Phyrexian Arena: "At the beginning of your upkeep, you draw a card and
/// you lose 1 life."
pub(super) fn arena() -> CardIndex {
    card_index("ee579a32-a048-4335-b966-231ba731cdea")
}

/// Grizzly Bears: a plain creature card.
pub(super) fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

// ---------------------------------------------------------------------------
// Boards
// ---------------------------------------------------------------------------

/// A duel over a Forest library, at player 0's first main phase (turn 1), with
/// the boards and opening hands as given.
#[track_caller]
pub(super) fn duel(
    seed: u64,
    p0_board: &[CardIndex],
    p0_hand: &[CardIndex],
    p1_board: &[CardIndex],
    p1_hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, p0_board)
        .hand(0, p0_hand)
        .battlefield(1, p1_board)
        .hand(1, p1_hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    assert_eq!(engine.state().turn.number, 1, "the first main phase");
    engine
}

/// The board for a single draw in the main phase: the Tome and the four lands
/// that pay its {4}.
pub(super) fn tome_board() -> Vec<CardIndex> {
    vec![tome(), forest(), forest(), forest(), forest()]
}

// ---------------------------------------------------------------------------
// Looking at the table
// ---------------------------------------------------------------------------

/// The cards in a seat's hand.
pub(super) fn hand(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(seat)).clone()
}

/// The cards in a seat's graveyard.
pub(super) fn graveyard(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .clone()
}

/// A seat's library, bottom first: the top card is the last one.
pub(super) fn library(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .clone()
}

/// The top `n` cards of a seat's library, the top card first.
pub(super) fn top(engine: &Engine<RegistryLookup>, seat: PlayerId, n: usize) -> Vec<ObjectId> {
    library(engine, seat)
        .iter()
        .rev()
        .take(n)
        .copied()
        .collect()
}

/// The same objects, in sorted order, to compare as sets.
pub(super) fn sorted(objects: &[ObjectId]) -> Vec<ObjectId> {
    let mut out = objects.to_vec();
    out.sort_unstable();
    out
}

/// The card an object is.
pub(super) fn card_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<CardIndex> {
    engine
        .state()
        .object(object)
        .and_then(|o| o.card)
        .map(|c| c.index)
}

/// Every permanent of `seat` that is `card`, on the battlefield.
pub(super) fn permanents(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Cards a seat has drawn this turn, as the engine counts them: a draw a
/// replacement took away is not one.
pub(super) fn drawn_this_turn(engine: &Engine<RegistryLookup>, seat: PlayerId) -> u32 {
    engine.state().per_turn.draws[usize::from(seat.get())]
}

// ---------------------------------------------------------------------------
// The question the engine asks about a draw
// ---------------------------------------------------------------------------

/// A `Pending::ChooseCards`, taken apart.
#[derive(Debug, Clone)]
pub(super) struct Question {
    /// Who is asked: the player drawing, always.
    pub player: PlayerId,
    /// What may be named.
    pub options: Vec<ObjectId>,
    /// How few.
    pub min: u8,
    /// How many.
    pub max: u8,
    /// What it is about.
    pub prompt: ChoicePrompt,
}

/// Whether the engine is waiting on a card question.
pub(super) fn is_asking(engine: &Engine<RegistryLookup>) -> bool {
    matches!(engine.pending(), Pending::ChooseCards { .. })
}

/// The card question on the table.
#[track_caller]
pub(super) fn question(engine: &Engine<RegistryLookup>) -> Question {
    match engine.pending() {
        Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt,
            ..
        } => Question {
            player: *player,
            options: options.clone(),
            min: *min,
            max: *max,
            prompt: *prompt,
        },
        other => panic!("expected a card question, got {other:?}"),
    }
}

/// Answers the card question on the table as the player it is put to.
#[track_caller]
pub(super) fn reply(engine: &mut Engine<RegistryLookup>, objects: &[ObjectId]) {
    let asked = question(engine);
    engine
        .apply(
            asked.player,
            PlayerAction::ChooseObjects {
                objects: objects.to_vec(),
            },
        )
        .expect("the engine takes an answer to its own question");
}

// ---------------------------------------------------------------------------
// Playing on
// ---------------------------------------------------------------------------

/// Passes priority (both players) until the stack is empty, or until a card
/// question is asked about something the stack is resolving.
#[track_caller]
pub(super) fn run_stack(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..60 {
        match engine.pending().clone() {
            Pending::ChooseCards { .. } => return,
            Pending::Priority { player, .. } => {
                if stack_is_empty(engine) {
                    return;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the stack resolves: {other:?}"),
        }
    }
    panic!("the stack never emptied: {:?}", engine.pending());
}

/// Plays on, passing priority and declaring no attackers and no blockers,
/// until `stop` holds or a card question is asked. `stop` is tried first.
/// Says whether a question stopped it.
#[track_caller]
pub(super) fn play_until(
    engine: &mut Engine<RegistryLookup>,
    stop: impl Fn(&Engine<RegistryLookup>) -> bool,
) -> bool {
    for _ in 0..400 {
        if stop(engine) {
            return false;
        }
        match engine.pending().clone() {
            Pending::ChooseCards { .. } => return true,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => engine
                .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                .unwrap(),
            Pending::ChooseBlockers { player, .. } => engine
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .unwrap(),
            other => panic!(
                "unexpected while playing on, turn {:?}: {other:?}",
                engine.state().turn
            ),
        }
    }
    panic!("never reached the stop: {:?}", engine.pending());
}

/// The first main phase of `turn`, played by `seat`.
pub(super) fn first_main_of(seat: PlayerId, turn: u32) -> impl Fn(&Engine<RegistryLookup>) -> bool {
    move |e| {
        e.state().turn.number == turn
            && e.state().turn.active == seat
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
    }
}

/// Whether the engine stands in `seat`'s draw step.
pub(super) fn in_draw_step_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> bool {
    engine.state().turn.active == seat && engine.state().turn.step == Step::Draw
}

// ---------------------------------------------------------------------------
// Things that draw
// ---------------------------------------------------------------------------

/// Draws a card with the first untapped Jayemdae Tome player 0 has, off the
/// mana already floating, and plays the stack on up to the first question.
/// "{4}, {T}: Draw a card": one draw, which is what a replacement is asked
/// about.
#[track_caller]
pub(super) fn tome_draw(engine: &mut Engine<RegistryLookup>) {
    activate(engine, P0, tome(), 0);
    run_stack(engine);
}

/// Casts Ancestral Recall from player 0's hand at `target` (tapping all of
/// player 0's mana for it) and plays the stack on up to the first question.
/// "Target player draws three cards": three draws, each its own event.
#[track_caller]
pub(super) fn recall_at(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    cast_from_hand(engine, P0, recall());
    engine
        .apply(P0, PlayerAction::ChoosePlayer(target))
        .expect("the spell names a player");
    run_stack(engine);
}

/// Lights Aladdin's Lamp for `x`: activates it off the mana floating, names
/// X, and lets the ability resolve. Checks that X is announced with a lowest
/// value of one ("X can't be 0") and nothing else about it.
#[track_caller]
pub(super) fn light_the_lamp(engine: &mut Engine<RegistryLookup>, x: u32) {
    activate(engine, P0, lamp(), 0);
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("{{X}} is announced: {:?}", engine.pending())
    };
    assert_eq!(min, 1, "\"X can't be 0\"");
    assert!(max >= x, "the pool pays for X = {x}: it pays up to {max}");
    engine
        .apply(P0, PlayerAction::ChooseNumber(x))
        .expect("X is announced");
    run_stack(engine);
    assert!(
        !is_asking(engine) && stack_is_empty(engine),
        "lighting the Lamp draws nothing and asks nothing: {:?}",
        engine.pending()
    );
}

/// Taps the given mana sources of player 0 and nothing else: mana of one kind
/// of land, for a test that spends it in parts.
#[track_caller]
pub(super) fn tap_lands_of(engine: &mut Engine<RegistryLookup>, lands: &[ObjectId]) {
    let lands = lands.to_vec();
    tap_mana_where(engine, P0, move |id| lands.contains(&id));
}
