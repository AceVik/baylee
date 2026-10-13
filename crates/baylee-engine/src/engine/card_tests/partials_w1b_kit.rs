//! Helpers for the wave-1 partial-card tests, pair B: walking a game to the
//! question a card asks, answering it, and reading what a player would see
//! afterwards.
//!
//! Nothing here knows a card. Every test that uses it states the printed
//! sentence it plays and asserts what that sentence says; this file only
//! carries the plumbing that `pass_until` deliberately does not (it answers a
//! fixed list of questions and panics on every other one, which is right for
//! a test that is not about a question and wrong for the ones that are).

#[allow(clippy::wildcard_imports)] // the card tests' own vocabulary
use super::*;

pub(super) const P0: PlayerId = PlayerId::new(0);
pub(super) const P1: PlayerId = PlayerId::new(1);
pub(super) const P2: PlayerId = PlayerId::new(2);

/// The indices of `source`'s abilities the seat holding priority is offered,
/// ascending — which is printed order, so a test that does not want to guess
/// where a face keeps its abilities reads them off the offer: the cheapest
/// ability is the first one offered with nothing floating, a loyalty ability
/// is the Nth of those offered at enough loyalty.
#[track_caller]
pub(super) fn offered(engine: &Engine<RegistryLookup>, source: ObjectId) -> Vec<u32> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let mut indices: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(id, _)| *id == source)
        .map(|(_, index)| *index)
        .collect();
    indices.sort_unstable();
    indices.dedup();
    indices
}

/// Activates ability `ability_index` of the object `source`, as `seat`.
#[track_caller]
pub(super) fn activate_object(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    source: ObjectId,
    ability_index: u32,
) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap_or_else(|err| {
            panic!("ability {ability_index} of {source:?} was refused to {seat:?}: {err:?}")
        });
}

/// Answers the entry question a land placed on the starting battlefield still
/// asks ("As this enters, you may pay N life. If you don't, it enters
/// tapped"), `pay` being the answer, for as long as that is the question.
/// Anything else asked is left alone.
#[track_caller]
pub(super) fn answer_the_entry_question(engine: &mut Engine<RegistryLookup>, pay: bool) {
    while let Pending::YesNo {
        player,
        prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { .. },
        ..
    } = engine.pending().clone()
    {
        engine
            .apply(player, PlayerAction::YesNo(pay))
            .expect("the entry question was asked, so it can be answered");
    }
}

/// Passes priority, declares nobody and answers each question it can with
/// `answer`, until `done` holds.
///
/// `answer` sees every question first and returns the action to take, or
/// `None` to let the default stand (pass priority, declare no attackers, no
/// blockers). Any other question `answer` leaves alone is a panic naming it,
/// because a test that did not expect a question has found a defect.
#[track_caller]
pub(super) fn drive(
    engine: &mut Engine<RegistryLookup>,
    done: impl Fn(&Engine<RegistryLookup>) -> bool,
    mut answer: impl FnMut(&Engine<RegistryLookup>, &Pending) -> Option<PlayerAction>,
) {
    for _ in 0..300 {
        if done(engine) {
            return;
        }
        let pending = engine.pending().clone();
        let Some(seat) = pending.asked() else {
            panic!("the game ended while waiting: {pending:?}")
        };
        let action = answer(engine, &pending).unwrap_or_else(|| match &pending {
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
            other => panic!("nothing in this test answers {other:?}"),
        });
        engine
            .apply(seat, action.clone())
            .unwrap_or_else(|err| panic!("{action:?} was refused in {pending:?}: {err:?}"));
    }
    panic!("never settled; still at {:?}", engine.pending());
}

/// Leaves the turn `engine` is in and walks on, passing priority and declaring
/// nobody, until `seat` holds priority in the first main phase of a *later*
/// turn. Unlike `walk_to_own_main`, it does not return at once for a seat that
/// is already in its own first main phase, and it has the room to cross a
/// table of three.
#[track_caller]
pub(super) fn walk_to_next_own_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let began = engine.state().turn.number;
    drive(
        engine,
        |e| {
            let turn = &e.state().turn;
            turn.number > began
                && turn.active == seat
                && matches!(turn.phase, Phase::FirstMain)
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == seat)
        },
        |_, _| None,
    );
}

/// Lets the engine take one step: the player holding priority passes it once.
/// State written behind the engine's back (the dev moves of cards between
/// zones) shows in the characteristics statics give only after the engine has
/// run a pass of its own (CR 113.6).
#[track_caller]
pub(super) fn let_the_engine_step(engine: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::PassPriority)
        .expect("passing priority is always legal");
}

/// Walks to `attacker`'s declaration of attackers (it is `attacker`'s turn),
/// declares `attackers` against `defender`, and lets `attacker` pass priority,
/// so that the next player holds priority in the declare attackers step with
/// the attack on the table. The way to act in an opponent's combat.
#[track_caller]
pub(super) fn attack_on_their_turn(
    engine: &mut Engine<RegistryLookup>,
    attacker: PlayerId,
    attackers: &[ObjectId],
    defender: PlayerId,
) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == attacker),
    );
    engine
        .apply(
            attacker,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(defender)))
                    .collect(),
            },
        )
        .unwrap_or_else(|err| panic!("{attackers:?} may attack {defender:?}: {err:?}"));
    let_the_engine_step(engine);
}

/// Passes priority and answers questions with `answer` until the stack is
/// empty again and somebody holds priority: whatever was put on it has
/// resolved. Unlike [`settle`], it does not insist the same seat holds that
/// priority, which on an opponent's turn is not the seat that acted (the
/// active player receives priority first after a resolution).
#[track_caller]
pub(super) fn resolve_the_stack(
    engine: &mut Engine<RegistryLookup>,
    answer: impl FnMut(&Engine<RegistryLookup>, &Pending) -> Option<PlayerAction>,
) {
    drive(
        engine,
        |e| stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. }),
        answer,
    );
}

/// Answers a `ChooseCards` question with those of `wanted` it offers (at most
/// `max` of them); every other question is left to the default.
pub(super) fn choose_cards(pending: &Pending, wanted: &[ObjectId]) -> Option<PlayerAction> {
    let Pending::ChooseCards { options, max, .. } = pending else {
        return None;
    };
    let objects = options
        .iter()
        .copied()
        .filter(|id| wanted.contains(id))
        .take(usize::from(*max))
        .collect();
    Some(PlayerAction::ChooseObjects { objects })
}

/// Answers a `ChooseTargets` question with `objects` and `players`; every
/// other question is left to the default.
pub(super) fn choose_targets(
    pending: &Pending,
    objects: &[ObjectId],
    players: &[PlayerId],
) -> Option<PlayerAction> {
    matches!(pending, Pending::ChooseTargets { .. }).then(|| PlayerAction::ChooseTargets {
        objects: objects.to_vec(),
        players: players.to_vec(),
    })
}

/// Settles the stack: everything on it resolves and `seat` is back at a quiet
/// priority, with `answer` taking the questions on the way.
#[track_caller]
pub(super) fn settle(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    answer: impl FnMut(&Engine<RegistryLookup>, &Pending) -> Option<PlayerAction>,
) {
    drive(engine, |e| at_rest(e, seat), answer);
}

/// Walks to the declare-attackers question and sends `attackers` at
/// `defender`, every one of them.
#[track_caller]
pub(super) fn declare_attack(
    engine: &mut Engine<RegistryLookup>,
    attackers: &[ObjectId],
    defender: PlayerId,
) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player,
        attackers: able,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the declaration")
    };
    for attacker in attackers {
        assert!(
            able.contains(attacker),
            "{attacker:?} may attack, offered {able:?}"
        );
    }
    assert!(
        defenders.contains(&Defender::Player(defender)),
        "{defender:?} may be attacked, offered {defenders:?}"
    );
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(defender)))
                    .collect(),
            },
        )
        .expect("the attack came out of the list that offered it");
}

/// Puts `top_first` on top of `seat`'s library in that order, out of the cards
/// the seat holds in hand (the harness' way: a library cannot be seeded any
/// other). Returns the objects, top card first.
#[track_caller]
pub(super) fn stack_library(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    top_first: &[CardIndex],
) -> Vec<ObjectId> {
    let mut objects: Vec<ObjectId> = top_first
        .iter()
        .rev()
        .map(|card| hand_to_library_top(engine, seat, *card))
        .collect();
    objects.reverse();
    objects
}

/// `seat`'s life total.
pub(super) fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

/// The objects in `seat`'s graveyard.
pub(super) fn graveyard_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .clone()
}

/// The objects in `seat`'s library, bottom first, so the top card is last.
pub(super) fn library_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .clone()
}

/// The zone `object` is in.
#[track_caller]
pub(super) fn zone_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> Zone {
    engine.state().object(object).expect("an object").zone
}

/// The controller of a permanent.
#[track_caller]
pub(super) fn controller_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> PlayerId {
    engine.state().object(object).expect("an object").controller
}

/// The owner of an object.
#[track_caller]
pub(super) fn owner_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> PlayerId {
    engine.state().object(object).expect("an object").owner
}

/// The face a double-faced permanent shows: 0 the front, 1 the back.
#[track_caller]
pub(super) fn face_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> u8 {
    engine.state().object(object).expect("an object").face_index
}

/// Damage marked on an object this turn.
#[track_caller]
pub(super) fn damage_on(engine: &Engine<RegistryLookup>, object: ObjectId) -> u16 {
    engine.state().object(object).expect("an object").damage
}

/// The colours an object has after the layers.
#[track_caller]
pub(super) fn colors_of(
    engine: &Engine<RegistryLookup>,
    object: ObjectId,
) -> baylee_core::color::ColorSet {
    engine
        .state()
        .object(object)
        .expect("an object")
        .characteristics()
        .colors
}

/// Whether an object has the subtype after the layers.
#[track_caller]
pub(super) fn has_subtype(
    engine: &Engine<RegistryLookup>,
    object: ObjectId,
    subtype: baylee_core::ids::SubtypeId,
) -> bool {
    engine
        .state()
        .object(object)
        .expect("an object")
        .characteristics()
        .subtypes
        .contains(subtype)
}

/// Whether an object has the supertype after the layers.
#[track_caller]
pub(super) fn has_supertype(
    engine: &Engine<RegistryLookup>,
    object: ObjectId,
    supertype: SupertypeSet,
) -> bool {
    engine
        .state()
        .object(object)
        .expect("an object")
        .characteristics()
        .supertypes
        .contains(supertype)
}

/// Whether the object is on the battlefield.
#[track_caller]
pub(super) fn is_on_battlefield(engine: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    zone_of(engine, object) == Zone::Battlefield
}

/// Puts a double-faced permanent on its back face where it stands, the
/// harness' way (the same placement `placed_face` makes).
#[track_caller]
pub(super) fn turn_over(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    object: ObjectId,
    card: CardIndex,
) {
    let def = baylee_cards::by_index(card).expect("in the pool");
    assert!(
        engine
            .dev_state_mut(seat)
            .expect("the harness may set boards up")
            .transform(object, def, 1),
        "placed on its back face"
    );
    engine.refresh_offer();
}

/// Puts exactly `amount` loyalty counters on `object`, the harness' way, and
/// refreshes the offer: a planeswalker placed on the battlefield rather than
/// cast does not enter with a printed loyalty a test can rely on.
#[track_caller]
pub(super) fn set_loyalty(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    object: ObjectId,
    amount: u16,
) {
    engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up")
        .object_mut(object)
        .expect("an object")
        .counters
        .set(CounterKind::Loyalty, amount);
    engine.refresh_offer();
}

/// The loyalty counters on `object`.
#[track_caller]
pub(super) fn loyalty(engine: &Engine<RegistryLookup>, object: ObjectId) -> u16 {
    engine
        .state()
        .object(object)
        .expect("an object")
        .counters
        .get(CounterKind::Loyalty)
}

/// Answers the target question in front of the engine with the one object
/// `target` and stops there: the ability or spell is on the stack and nobody
/// has passed, so a test can act in response.
#[track_caller]
pub(super) fn name_the_target(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!("expected a target question, got {:?}", engine.pending())
    };
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("the target was on the menu");
}
