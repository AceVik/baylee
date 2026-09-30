//! State triggers (CR 603.8): "When you control no Islands, …" triggers as
//! soon as the game state matches its condition, and not again until the
//! ability has left the stack.
//!
//! Played with creatures built for it, one sentence each, on a board whose
//! only Island is moved about by hand.

use super::synthetic::{SyntheticLookup, creature_face, keep_mulligans, permanents, preset};
use super::*;
use crate::event::Cause;
use crate::zone::ZonePosition;
use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Condition, Coverage, Effect, FaceDef, Filter,
    KeywordSet, TargetReq, TargetSpec, Trigger, triggered,
};
use baylee_core::color::ColorSet;
use baylee_core::generated::subtypes::land;
use baylee_core::ids::CardIndex;

// ---------------------------------------------------------------- fixtures

/// A 5/5 with "When you control no Islands, sacrifice this creature."
const SERPENT: u32 = 1140;
/// A 1/1 with "When you control no Islands, you gain 1 life", which does
/// nothing to end the state it triggers on.
const WATCHER: u32 = 1141;
/// A 1/1 with "When you control no Islands, this creature deals 1 damage
/// to any target": a state trigger that asks a question before it goes on
/// the stack.
const PINGER: u32 = 1142;

static ISLAND: Filter = Filter::HasSubtype(land::ISLAND);
static NO_ISLANDS: Condition = Condition::ControlCountAtMost(&ISLAND, 0);
static SERPENT_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::State(&NO_ISLANDS),
    &[Effect::SacrificeSelf]
)];
static WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::State(&NO_ISLANDS),
    &[Effect::GainLife {
        amount: Amount::Fixed(1)
    }]
)];
static PINGER_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::State(&NO_ISLANDS),
    &[Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::AnyTarget
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyTarget))
)];

fn body(
    index: u32,
    name: &'static str,
    power: i16,
    toughness: i16,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([FaceDef {
            abilities,
            ..creature_face(name, power, toughness, &[])
        }])),
        color_identity: ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

fn lookup() -> SyntheticLookup {
    SyntheticLookup::new(vec![
        body(SERPENT, "Serpent", 5, 5, SERPENT_ABILITIES),
        body(WATCHER, "Watcher", 1, 1, WATCHER_ABILITIES),
        body(PINGER, "Pinger", 1, 1, PINGER_ABILITIES),
    ])
}

/// The pool's Island.
fn island() -> u32 {
    baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
        .expect("Island exists")
        .index
        .get()
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn start(mine: &[u32]) -> Engine<SyntheticLookup> {
    let mut engine = Engine::new(&preset(41, mine), lookup()).unwrap();
    keep_mulligans(&mut engine);
    engine
}

/// The abilities on the stack whose source is `source`.
fn on_stack_from(engine: &Engine<SyntheticLookup>, source: ObjectId) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.ability)
                .is_some_and(|loc| loc.source == source)
        })
        .count()
}

/// Whoever holds priority passes it.
fn pass(engine: &mut Engine<SyntheticLookup>) {
    match engine.pending().clone() {
        Pending::Priority { player, .. } => {
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        }
        other => panic!("expected a priority question, got {other:?}"),
    }
}

/// Moves `id` to `to` the way a dev command rewrites a board: no event of
/// the game's own, so what follows is the state trigger reading the state.
fn move_to(engine: &mut Engine<SyntheticLookup>, id: ObjectId, to: ZoneLocation) {
    engine
        .dev_state_mut(ME)
        .expect("a test seat has dev commands")
        .move_object(id, to, ZonePosition::Top, Cause::DevCommand)
        .expect("the object moves");
}

fn life(engine: &Engine<SyntheticLookup>) -> i32 {
    engine.state().players[0].life
}

// ------------------------------------------------------------------- tests

/// While its controller has an Island the ability is quiet; once the Island
/// is gone it triggers before anybody next receives priority, and resolving
/// it sacrifices the creature.
#[test]
fn a_state_trigger_triggers_once_the_state_matches() {
    let mut engine = start(&[SERPENT, island()]);
    let serpent = permanents(&engine, SERPENT)[0];
    let isle = permanents(&engine, island())[0];
    // A whole round of passes with the Island there: the step ends and
    // nothing triggered.
    let step = engine.state().turn.step;
    pass(&mut engine);
    pass(&mut engine);
    assert_ne!(engine.state().turn.step, step, "the round ended the step");
    assert_eq!(
        on_stack_from(&engine, serpent),
        0,
        "with an Island under its controller's control, nothing triggers"
    );
    // Moved while the active player holds priority, so that their pass is
    // the first moment the engine looks: a dev move runs no rules of its own.
    move_to(&mut engine, isle, ZoneLocation::Hand(ME));
    pass(&mut engine);
    assert_eq!(
        on_stack_from(&engine, serpent),
        1,
        "no Islands: the ability triggers and is on the stack (CR 603.8)"
    );
    while on_stack_from(&engine, serpent) > 0 {
        pass(&mut engine);
    }
    assert!(
        permanents(&engine, SERPENT).is_empty(),
        "the ability resolved and sacrificed its source"
    );
}

/// The ability does not trigger again while it is on the stack, however
/// many times priority passes, and once it has resolved it triggers again
/// at once when the state still matches.
#[test]
fn a_state_trigger_waits_for_its_ability_to_leave_the_stack() {
    let mut engine = start(&[WATCHER]);
    let watcher = permanents(&engine, WATCHER)[0];
    assert_eq!(
        on_stack_from(&engine, watcher),
        1,
        "no Islands from the start: triggered before the first priority"
    );
    pass(&mut engine);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the other player holds priority over the same ability"
    );
    assert_eq!(
        on_stack_from(&engine, watcher),
        1,
        "not again while the ability is on the stack (CR 603.8)"
    );
    assert_eq!(life(&engine), 20);
    pass(&mut engine);
    assert_eq!(life(&engine), 21, "the ability resolved");
    assert_eq!(
        on_stack_from(&engine, watcher),
        1,
        "resolved, and the state still matches: it triggers again"
    );
}

/// The condition is what makes the ability trigger, not an "if" it asks
/// again as it resolves (CR 603.4 is a different sentence): an Island that
/// arrives while the ability is on the stack does not stop it.
#[test]
fn a_state_trigger_resolves_even_after_the_state_is_gone() {
    let mut engine = start(&[SERPENT, island()]);
    let serpent = permanents(&engine, SERPENT)[0];
    let isle = permanents(&engine, island())[0];
    move_to(&mut engine, isle, ZoneLocation::Hand(ME));
    pass(&mut engine);
    assert_eq!(on_stack_from(&engine, serpent), 1);
    let in_hand = engine.state().zones.list(ZoneLocation::Hand(ME))[0];
    move_to(&mut engine, in_hand, ZoneLocation::Battlefield);
    assert_eq!(
        permanents(&engine, island()).len(),
        1,
        "the Island is back under its owner's control"
    );
    while on_stack_from(&engine, serpent) > 0 {
        pass(&mut engine);
    }
    assert!(
        permanents(&engine, SERPENT).is_empty(),
        "the ability resolved as it was, and sacrificed its source"
    );
}

/// Two of them trigger together and each asks for its target in turn. The
/// second waits in line while the first is asked, and does not trigger a
/// second time from there: it is on its way to the stack already.
#[test]
fn a_state_trigger_waiting_to_be_put_on_the_stack_does_not_trigger_again() {
    let mut engine = start(&[PINGER, PINGER]);
    let pingers = permanents(&engine, PINGER);
    let mut asked = 0;
    while let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    {
        assert!(player_options.contains(&THEM));
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![THEM],
                },
            )
            .unwrap();
        asked += 1;
        assert!(asked <= 2, "asked for a third target");
    }
    assert_eq!(asked, 2, "each of the two asked once");
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
    for pinger in pingers {
        assert_eq!(on_stack_from(&engine, pinger), 1);
    }
}
