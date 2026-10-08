//! A game that repeats a *shape* without repeating a *situation* is not an
//! endless loop, and the `RunOnceThenBreak` house rule must not withhold its
//! triggers.
//!
//! The rule withholds every new trigger until the stack drains once a loop
//! is detected (`Engine::on_loop_detected`), so a false detection would look
//! exactly like an enters trigger that never fired. Detection needs the
//! answer-time signature (`Engine::apply`: `loop_signature`, the answering
//! seat and the pass count) to come back to a value it had before, twice,
//! a period apart (`crate::loops`). These are the shapes most likely to
//! do that without being a loop: two copies of one creature entering in
//! turn, one card reanimated twice, and a persist creature dying and coming
//! back. Each is played with the real cards, every answer's signature is
//! recorded the way `apply` builds it, and the engine's own detector is run
//! over them with every answer sampled (`Recorder::detected`).

use super::testkit::{
    Duel, RegistryLookup, card_index, keep_mulligans, tap_all_mana, walk_to_own_main,
};
use super::*;
use crate::event::{Cause, GameEvent};
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::{CardIndex, ObjectId};

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn merchant_of_secrets() -> CardIndex {
    card_index("f6aebd42-0150-4741-84c2-4c85893640e9")
}
fn mulldrifter() -> CardIndex {
    card_index("24d0f5e7-0d9e-4b76-900e-a7274e80312d")
}
fn reanimate() -> CardIndex {
    card_index("a044474a-cd72-4e9d-bd8d-a08f2de9cdc0")
}
fn animate_dead() -> CardIndex {
    card_index("c0d8fef4-65f4-4769-982d-b397d2b7e977")
}
fn kitchen_finks() -> CardIndex {
    card_index("5470dcfa-4eff-43da-abf7-19922841f719")
}

const P0: PlayerId = PlayerId::new(0);

/// Plays a game forward, recording the signature `Engine::apply` samples
/// before every answer it gives.
struct Recorder {
    signatures: Vec<u64>,
}

impl Recorder {
    fn answer(
        &mut self,
        engine: &mut Engine<RegistryLookup>,
        player: PlayerId,
        action: PlayerAction,
    ) {
        // The formula `Engine::apply` hands the loop watch.
        self.signatures.push(
            engine.state.loop_signature()
                ^ u64::from(player.get()).rotate_left(37)
                ^ u64::from(engine.passes).rotate_left(53),
        );
        engine.apply(player, action).unwrap();
    }

    /// Answers passively (a target question with `aim`) until `done`.
    fn until(
        &mut self,
        engine: &mut Engine<RegistryLookup>,
        aim: Option<ObjectId>,
        done: impl Fn(&Engine<RegistryLookup>) -> bool,
    ) {
        for _ in 0..200 {
            if done(engine) {
                return;
            }
            let (player, action) = match engine.pending().clone() {
                Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
                Pending::ChooseAttackers { player, .. } => {
                    (player, PlayerAction::DeclareAttackers { attackers: vec![] })
                }
                Pending::ChooseBlockers { player, .. } => {
                    (player, PlayerAction::DeclareBlockers { blockers: vec![] })
                }
                Pending::YesNo { player, .. } => (player, PlayerAction::YesNo(true)),
                Pending::ChooseTargets { player, .. } => (
                    player,
                    PlayerAction::ChooseTargets {
                        objects: aim.into_iter().collect(),
                        players: vec![],
                    },
                ),
                other => panic!("unexpected: {other:?}"),
            };
            self.answer(engine, player, action);
        }
        panic!("never settled");
    }

    fn cast(
        &mut self,
        engine: &mut Engine<RegistryLookup>,
        card: CardIndex,
        aim: Option<ObjectId>,
    ) {
        tap_all_mana(engine, P0);
        let spell = super::testkit::in_hand(engine, P0, card).expect("in hand");
        self.answer(engine, P0, PlayerAction::CastSpell { card: spell });
        self.until(engine, aim, settled);
    }

    /// The period the engine's own detector reports over these answers, if
    /// any, with **every** answer sampled. The game samples only past
    /// `LoopWatch::WATCH_AFTER` answers and then one in `SAMPLE_EVERY`; a
    /// stream that this, the strictest sampling, does not call a loop is
    /// not one under any coarser sampling either. `None` is the guarantee.
    ///
    /// Not "no signature repeats": a cast and the target question it asks
    /// are two answers to one unchanged situation (the card goes to the
    /// stack once the targets are in), so a spell with a target shows its
    /// signature twice in a row. One repeat is what the detector puts on
    /// probation and the next answer clears.
    fn detected(&self) -> Option<u64> {
        let mut watch = crate::loops::LoopWatch::default();
        self.signatures
            .iter()
            .find_map(|signature| watch.step(Some(*signature)))
    }
}

/// The stack is empty and somebody holds priority: not merely empty while a
/// spell's targets are being chosen, before it is on the stack at all.
fn settled(engine: &Engine<RegistryLookup>) -> bool {
    super::testkit::stack_is_empty(engine) && matches!(engine.pending(), Pending::Priority { .. })
}

fn loops_detected(engine: &Engine<RegistryLookup>) -> usize {
    engine
        .journal()
        .entries()
        .iter()
        .filter(|e| matches!(e.event, GameEvent::LoopDetected { .. }))
        .count()
}

fn hand(engine: &Engine<RegistryLookup>) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(P0)).len()
}

/// Two Merchants of Secrets cast one after the other: the same enters
/// trigger on two objects, and each draws.
#[test]
fn two_copies_of_one_enters_trigger_are_not_a_loop() {
    let mut engine = Duel::new(5, island())
        .hand(0, &[merchant_of_secrets(), merchant_of_secrets()])
        .battlefield(0, &[island(); 6])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));
    let mut rec = Recorder { signatures: vec![] };
    let before = hand(&engine);
    rec.cast(&mut engine, merchant_of_secrets(), None);
    rec.cast(&mut engine, merchant_of_secrets(), None);
    assert_eq!(hand(&engine), before - 2 + 2, "both enter triggers drew");
    assert_eq!(loops_detected(&engine), 0);
    assert_eq!(rec.detected(), None, "the detector called this a loop");
}

/// One Mulldrifter reanimated twice, by Reanimate and then by Animate Dead,
/// after going back to the graveyard in between.
#[test]
fn one_card_reanimated_twice_is_not_a_loop() {
    let mut engine = Duel::new(5, island())
        .hand(0, &[reanimate(), animate_dead(), mulldrifter()])
        .battlefield(0, &[swamp(); 3])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));
    let mull = super::testkit::in_hand(&engine, P0, mulldrifter()).unwrap();
    let to_graveyard = |engine: &mut Engine<RegistryLookup>| {
        engine
            .dev_state_mut(P0)
            .unwrap()
            .move_object(
                mull,
                ZoneLocation::Graveyard(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    };
    to_graveyard(&mut engine);
    let mut rec = Recorder { signatures: vec![] };
    let before = hand(&engine);
    rec.cast(&mut engine, reanimate(), Some(mull));
    assert_eq!(
        hand(&engine),
        before - 1 + 2,
        "Reanimate's Mulldrifter drew two"
    );
    to_graveyard(&mut engine);
    // Fresh mana for the second spell, as a new turn would give it.
    let state = engine.dev_state_mut(P0).unwrap();
    for land in state.zones.list(ZoneLocation::Battlefield).clone() {
        state.set_tapped(land, false);
    }
    engine.refresh_offer();
    let before = hand(&engine);
    rec.cast(&mut engine, animate_dead(), Some(mull));
    assert_eq!(
        hand(&engine),
        before - 1 + 2,
        "Animate Dead's Mulldrifter drew two"
    );
    assert_eq!(loops_detected(&engine), 0);
    assert_eq!(rec.detected(), None, "the detector called this a loop");
}

/// Kitchen Finks dies, persists back with a -1/-1 counter (gaining 2 life
/// again), and dies for good.
#[test]
fn a_persist_return_is_not_a_loop() {
    let mut engine = Duel::new(5, forest())
        .hand(0, &[kitchen_finks()])
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));
    let finks = super::testkit::in_hand(&engine, P0, kitchen_finks()).unwrap();
    let mut rec = Recorder { signatures: vec![] };
    let life = engine.state().players[0].life;
    rec.cast(&mut engine, kitchen_finks(), None);
    assert_eq!(
        engine.state().players[0].life,
        life + 2,
        "it entered and gained 2"
    );
    for gained in [4, 4] {
        let dies = |engine: &mut Engine<RegistryLookup>| {
            engine
                .dev_state_mut(P0)
                .unwrap()
                .move_object(
                    finks,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        };
        dies(&mut engine);
        // The death is collected on the next pass through the machine.
        rec.answer(&mut engine, P0, PlayerAction::PassPriority);
        rec.until(&mut engine, None, settled);
        assert_eq!(engine.state().players[0].life, life + gained);
    }
    assert_eq!(loops_detected(&engine), 0);
    assert_eq!(rec.detected(), None, "the detector called this a loop");
}
