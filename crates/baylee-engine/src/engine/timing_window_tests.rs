//! Spells and abilities restricted to a part of the turn (CR 506.7, 506.7g),
//! and the history two cards of that kind ask about: which creatures
//! attacked this turn (CR 508.1), and which a player has controlled since
//! the turn began (CR 302.6).
//!
//! Played with cards built for it, one sentence each, walked through whole
//! turns: each window is shown open where the rule opens it and shut on
//! both sides.

use super::synthetic::{SyntheticLookup, creature_face, keep_mulligans, permanents, preset_both};
use super::*;
use crate::turn::{Phase, Step};
use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Condition, Cost, Coverage, Effect, FaceDef, Filter,
    KeywordSet, Modifier, StepKind, TargetReq, TargetSpec, activated, spell,
};
use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, Defender, PrintRef};
use baylee_core::preset::DeckEntry;
use baylee_core::types::TypeSet;

// ---------------------------------------------------------------- fixtures

/// A free instant: "You gain 1 life. Cast this spell only before the combat
/// damage step." Berserk's window.
const RECKLESS: u32 = 1150;
/// A 1/1 with Nettling Imp's sentence, less its word about Walls: "{T}:
/// Choose target creature the active player has controlled continuously
/// since the beginning of the turn. That creature attacks this turn if
/// able. Destroy it at the beginning of the next end step if it didn't
/// attack this turn. Activate only during an opponent's turn, before
/// attackers are declared."
const HECKLER: u32 = 1151;
/// A vanilla 2/2.
const BEAR: u32 = 1152;
/// A 1/1 with "{0}: You gain 1 life. Activate only during your upkeep."
const WAKER: u32 = 1153;
/// A vanilla 1/1 costing `{0}`, cast to have a creature that arrived this
/// turn.
const NEWBIE: u32 = 1154;

static RECKLESS_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::GainLife {
        amount: Amount::Fixed(1)
    }],
    condition = Some(Condition::BeforeStep(StepKind::CombatDamage))
)];

static HELD_SINCE_THE_TURN_BEGAN: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByActivePlayer,
    Filter::ControlledSinceTurnBegan,
]);
static HECKLER_ABILITIES: &[AbilityDef] = &[activated!(
    Cost::TAP,
    &[
        Effect::continuous(
            &Filter::This,
            Modifier::AttacksEachCombat,
            baylee_cards_dsl::Duration::UntilEndOfTurn
        ),
        Effect::AtNextEndStep {
            effects: &[Effect::IfEventObjectMatches {
                filter: &Filter::Not(&Filter::AttackedThisTurn),
                then: &[Effect::destroy(TargetSpec::EventObject)],
            }],
        },
    ],
    targets = Some(TargetReq::one(TargetSpec::Object(
        &HELD_SINCE_THE_TURN_BEGAN
    ))),
    condition = Some(Condition::All(&[
        Condition::OpponentsTurn,
        Condition::BeforeStep(StepKind::DeclareAttackers),
    ]))
)];

static WAKER_ABILITIES: &[AbilityDef] = &[activated!(
    Cost::FREE,
    &[Effect::GainLife {
        amount: Amount::Fixed(1)
    }],
    condition = Some(Condition::All(&[
        Condition::YourTurn,
        Condition::DuringStep(StepKind::Upkeep),
    ]))
)];

/// A printed `{0}`: a card with no mana cost at all cannot be cast
/// (CR 118.6).
const FREE: baylee_core::mana::ManaCost = baylee_core::mana::ManaCost::parse("{0}");

fn card(index: u32, face: &FaceDef, abilities: &'static [AbilityDef]) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([FaceDef { abilities, ..*face }])),
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
        card(
            RECKLESS,
            &FaceDef {
                name: "Reckless",
                mana_cost: FREE,
                types: TypeSet::INSTANT,
                ..FaceDef::DEFAULT
            },
            RECKLESS_ABILITIES,
        ),
        card(
            HECKLER,
            &creature_face("Heckler", 1, 1, &[]),
            HECKLER_ABILITIES,
        ),
        card(BEAR, &creature_face("Bear", 2, 2, &[]), &[]),
        card(WAKER, &creature_face("Waker", 1, 1, &[]), WAKER_ABILITIES),
        card(
            NEWBIE,
            &FaceDef {
                mana_cost: FREE,
                ..creature_face("Newbie", 1, 1, &[])
            },
            &[],
        ),
    ])
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

/// A game on turn 1, `ME` active, with these boards and `ME` holding `hand`.
fn start(mine: &[u32], theirs: &[u32], hand: &[u32]) -> Engine<SyntheticLookup> {
    let mut preset = preset_both(61, mine, theirs);
    preset.seats[0].starting_hand = Some(
        hand.iter()
            .map(|c| DeckEntry {
                card: CardIndex::new(*c),
                print: PrintRef::new(0),
            })
            .collect(),
    );
    let mut engine = Engine::new(&preset, lookup()).unwrap();
    keep_mulligans(&mut engine);
    engine
}

fn in_hand(engine: &Engine<SyntheticLookup>, index: u32) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(ME))
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index.get() == index))
        })
        .collect()
}

/// Where the turn stands, as a card names it: the step, and for a main
/// phase which of the two it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum At {
    Step(Step),
    FirstMain,
    SecondMain,
}

fn at(engine: &Engine<SyntheticLookup>) -> At {
    let turn = &engine.state().turn;
    match (turn.step, turn.phase) {
        (Step::Main, Phase::FirstMain) => At::FirstMain,
        (Step::Main, _) => At::SecondMain,
        (step, _) => At::Step(step),
    }
}

/// Plays on until `until` holds. Every priority is shown to `seen` before
/// it is passed; `attack` names the creatures that attack when their
/// controller is asked, and nobody blocks.
fn walk(
    engine: &mut Engine<SyntheticLookup>,
    until: impl Fn(&Engine<SyntheticLookup>) -> bool,
    mut seen: impl FnMut(&Engine<SyntheticLookup>, PlayerId, &LegalActions),
    attack: &[ObjectId],
) {
    for _ in 0..300 {
        if until(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, legal } => {
                seen(engine, player, &legal);
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                let defender = if player == ME { THEM } else { ME };
                let attackers = attack
                    .iter()
                    .filter(|a| {
                        engine.state().object(**a).is_some_and(|o| {
                            o.controller == player && o.zone == crate::zone::Zone::Battlefield
                        })
                    })
                    .map(|a| (*a, Defender::Player(defender)))
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
            other => panic!("unexpected on the way: {other:?}"),
        }
    }
    panic!("the walk never arrived");
}

fn turn_is(n: u32) -> impl Fn(&Engine<SyntheticLookup>) -> bool {
    move |e| e.state().turn.number == n
}

/// Whether, at each place `ME` or `THEM` held priority on the way, what
/// `offered` asks was true, in the order they came.
type Seen = Vec<(u32, At, bool)>;

fn assert_open(seen: &Seen, turn: u32, place: At, open: bool) {
    let found: Vec<bool> = seen
        .iter()
        .filter(|(n, a, _)| *n == turn && *a == place)
        .map(|(_, _, o)| *o)
        .collect();
    assert!(
        !found.is_empty(),
        "no priority on turn {turn} at {place:?}; seen: {seen:?}"
    );
    assert!(
        found.iter().all(|o| *o == open),
        "on turn {turn} at {place:?} the window should be {}; seen: {seen:?}",
        if open { "open" } else { "shut" }
    );
}

// ------------------------------------------------------------------- tests

/// "Cast this spell only before the combat damage step" (CR 506.7): offered
/// from the upkeep through the declare blockers step, and from the combat
/// damage step on, not.
#[test]
fn a_spell_cast_only_before_the_combat_damage_step_is_offered_until_that_step() {
    let mut engine = start(&[BEAR], &[], &[RECKLESS]);
    let bear = permanents(&engine, BEAR)[0];
    let spell = in_hand(&engine, RECKLESS)[0];
    let mut seen: Seen = Vec::new();
    walk(
        &mut engine,
        turn_is(2),
        |e, player, legal| {
            if player == ME {
                seen.push((
                    e.state().turn.number,
                    at(e),
                    legal.castable.contains(&spell),
                ));
            }
        },
        &[bear],
    );
    for open in [
        At::Step(Step::Upkeep),
        At::FirstMain,
        At::Step(Step::CombatBegin),
        At::Step(Step::DeclareAttackers),
        At::Step(Step::DeclareBlockers),
    ] {
        assert_open(&seen, 1, open, true);
    }
    for shut in [
        At::Step(Step::CombatDamage),
        At::Step(Step::CombatEnd),
        At::SecondMain,
        At::Step(Step::End),
    ] {
        assert_open(&seen, 1, shut, false);
    }
}

/// With no attackers there is no combat damage step to be before, and the
/// window closes as the declare attackers step ends (CR 506.7e).
#[test]
fn with_no_attackers_the_window_closes_as_the_declare_attackers_step_ends() {
    let mut engine = start(&[BEAR], &[], &[RECKLESS]);
    let spell = in_hand(&engine, RECKLESS)[0];
    let mut seen: Seen = Vec::new();
    walk(
        &mut engine,
        turn_is(2),
        |e, player, legal| {
            if player == ME {
                seen.push((
                    e.state().turn.number,
                    at(e),
                    legal.castable.contains(&spell),
                ));
            }
        },
        &[],
    );
    assert_open(&seen, 1, At::Step(Step::CombatBegin), true);
    assert_open(&seen, 1, At::Step(Step::CombatEnd), false);
    assert!(
        !seen
            .iter()
            .any(|(_, a, _)| matches!(a, At::Step(Step::DeclareBlockers | Step::CombatDamage))),
        "no attackers: the declare blockers and combat damage steps are skipped (CR 508.8)"
    );
}

/// The window is the engine's rule and not only the offer's: a cast
/// outside it is refused, and the same cast inside it is taken.
#[test]
fn a_spell_cast_outside_its_window_is_refused() {
    let mut engine = start(&[], &[], &[RECKLESS, RECKLESS]);
    let spells = in_hand(&engine, RECKLESS);
    walk(&mut engine, |e| at(e) == At::FirstMain, |_, _, _| {}, &[]);
    engine
        .apply(ME, PlayerAction::CastSpell { card: spells[0] })
        .expect("inside the window, the cast is taken");
    walk(
        &mut engine,
        |e| {
            at(e) == At::SecondMain
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == ME)
        },
        |_, _, _| {},
        &[],
    );
    assert!(
        engine
            .apply(ME, PlayerAction::CastSpell { card: spells[1] })
            .is_err(),
        "after the combat damage step's place in the turn, the cast is refused"
    );
    assert_eq!(in_hand(&engine, RECKLESS), vec![spells[1]]);
}

/// "Activate only during an opponent's turn, before attackers are declared"
/// (CR 506.7a, 506.7g): offered to the Heckler's controller on the other
/// seat's turn until the declare attackers step, and never on their own
/// turn, where the Heckler itself would be a legal target.
#[test]
fn an_ability_for_an_opponent_s_turn_before_attackers_is_offered_only_then() {
    let mut engine = start(&[BEAR], &[HECKLER], &[]);
    let bear = permanents(&engine, BEAR)[0];
    let heckler = permanents(&engine, HECKLER)[0];
    let mut seen: Seen = Vec::new();
    walk(
        &mut engine,
        turn_is(3),
        |e, player, legal| {
            if player == THEM {
                seen.push((
                    e.state().turn.number,
                    at(e),
                    legal.abilities.contains(&(heckler, 0)),
                ));
            }
        },
        &[bear],
    );
    for open in [
        At::Step(Step::Upkeep),
        At::FirstMain,
        At::Step(Step::CombatBegin),
    ] {
        assert_open(&seen, 1, open, true);
    }
    for shut in [
        At::Step(Step::DeclareAttackers),
        At::Step(Step::DeclareBlockers),
        At::SecondMain,
        At::Step(Step::End),
    ] {
        assert_open(&seen, 1, shut, false);
    }
    assert!(
        seen.iter().any(|(n, _, _)| *n == 2),
        "the Heckler's controller held priority on their own turn"
    );
    assert!(
        seen.iter().filter(|(n, _, _)| *n == 2).all(|(_, _, o)| !o),
        "never on their own turn: {seen:?}"
    );
}

/// "Activate only during your upkeep": the step and the turn, both.
#[test]
fn an_ability_for_your_upkeep_is_offered_in_that_step_alone() {
    let mut engine = start(&[WAKER], &[], &[]);
    let waker = permanents(&engine, WAKER)[0];
    let mut seen: Seen = Vec::new();
    walk(
        &mut engine,
        turn_is(3),
        |e, player, legal| {
            if player == ME {
                seen.push((
                    e.state().turn.number,
                    at(e),
                    legal.abilities.contains(&(waker, 0)),
                ));
            }
        },
        &[],
    );
    assert_open(&seen, 1, At::Step(Step::Upkeep), true);
    for shut in [
        At::FirstMain,
        At::Step(Step::CombatBegin),
        At::Step(Step::End),
    ] {
        assert_open(&seen, 1, shut, false);
    }
    assert_open(&seen, 2, At::Step(Step::Upkeep), false);
}

/// Waits for `THEM` to hold priority in `ME`'s first main phase, activates
/// the Heckler and returns the creatures it offers as targets, answering
/// with `pick` if one is given.
fn heckle(engine: &mut Engine<SyntheticLookup>, pick: Option<ObjectId>) -> Vec<ObjectId> {
    walk(
        engine,
        |e| {
            at(e) == At::FirstMain
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == THEM)
        },
        |_, _, _| {},
        &[],
    );
    let heckler = permanents(engine, HECKLER)[0];
    engine
        .apply(
            THEM,
            PlayerAction::ActivateAbility {
                source: heckler,
                ability_index: 0,
            },
        )
        .expect("the ability is offered in the active player's main phase");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the target question, got {:?}", engine.pending());
    };
    if let Some(target) = pick {
        engine
            .apply(
                THEM,
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .unwrap();
        while !engine.state().zones.list(ZoneLocation::Stack).is_empty() {
            let Pending::Priority { player, .. } = engine.pending().clone() else {
                panic!("expected priority, got {:?}", engine.pending());
            };
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        }
    }
    options
}

/// "A creature the active player has controlled continuously since the
/// beginning of the turn": one cast this turn is not among the targets,
/// and neither is the Heckler, which the active player does not control.
#[test]
fn a_creature_that_arrived_this_turn_is_not_one_held_since_the_turn_began() {
    let mut engine = start(&[BEAR], &[HECKLER], &[NEWBIE]);
    let bear = permanents(&engine, BEAR)[0];
    walk(&mut engine, |e| at(e) == At::FirstMain, |_, _, _| {}, &[]);
    let newbie = in_hand(&engine, NEWBIE)[0];
    engine
        .apply(ME, PlayerAction::CastSpell { card: newbie })
        .unwrap();
    walk(
        &mut engine,
        |e| permanents(e, NEWBIE).len() == 1,
        |_, _, _| {},
        &[],
    );
    let options = heckle(&mut engine, None);
    assert_eq!(
        options,
        vec![bear],
        "only the Bear has been under the active player's control since the turn began"
    );
}

/// The creature it names must attack, and one that did survives the end
/// step: attacking this turn is what the delayed destruction asks.
#[test]
fn a_heckled_creature_that_attacked_survives_the_end_step() {
    let mut engine = start(&[BEAR], &[HECKLER], &[]);
    let bear = permanents(&engine, BEAR)[0];
    heckle(&mut engine, Some(bear));
    walk(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { .. }),
        |_, _, _| {},
        &[],
    );
    assert!(
        engine
            .apply(ME, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "the Bear attacks this turn if able (CR 508.1d)"
    );
    walk(&mut engine, turn_is(2), |_, _, _| {}, &[bear]);
    assert_eq!(engine.state().players[1].life, 18, "the Bear attacked");
    assert_eq!(
        permanents(&engine, BEAR),
        vec![bear],
        "it attacked this turn, so the end step left it alone"
    );
}

/// One that could not attack is destroyed as the end step begins.
#[test]
fn a_heckled_creature_that_did_not_attack_is_destroyed_at_the_end_step() {
    let mut engine = start(&[BEAR], &[HECKLER], &[]);
    let bear = permanents(&engine, BEAR)[0];
    heckle(&mut engine, Some(bear));
    engine
        .dev_state_mut(ME)
        .expect("a test seat has dev commands")
        .set_tapped(bear, true);
    walk(
        &mut engine,
        |e| at(e) == At::Step(Step::End),
        |_, _, _| {},
        &[],
    );
    assert_eq!(
        permanents(&engine, BEAR),
        vec![bear],
        "still there as the end step begins"
    );
    walk(&mut engine, turn_is(2), |_, _, _| {}, &[]);
    assert!(
        permanents(&engine, BEAR).is_empty(),
        "tapped, it could not attack, and the delayed trigger destroyed it"
    );
}

/// "Attacked this turn" is about the object that was declared (CR 508.1):
/// the same card back on the battlefield is a new object that did not
/// (CR 400.7), and the record is the turn's own.
#[test]
fn attacked_this_turn_is_the_declared_object_s_and_this_turn_s() {
    let mut engine = start(&[BEAR, BEAR], &[], &[]);
    let bears = permanents(&engine, BEAR);
    let attacked = |e: &Engine<SyntheticLookup>, id: ObjectId| {
        let obj = e.state().object(id).expect("the bear");
        crate::eval::matches(&Filter::AttackedThisTurn, e.state(), obj, ME, id)
    };
    walk(
        &mut engine,
        |e| at(e) == At::SecondMain,
        |_, _, _| {},
        &bears[..1],
    );
    assert!(attacked(&engine, bears[0]), "declared as an attacker");
    assert!(!attacked(&engine, bears[1]), "never declared");
    // Out and back: a new object under the same id (CR 400.7).
    let state = engine.dev_state_mut(ME).expect("dev commands");
    state
        .move_object(
            bears[0],
            ZoneLocation::Hand(ME),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .unwrap();
    state
        .move_object(
            bears[0],
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .unwrap();
    assert!(
        !attacked(&engine, bears[0]),
        "the card that came back is a new object, which never attacked"
    );

    let mut engine = start(&[BEAR], &[], &[]);
    let bear = permanents(&engine, BEAR)[0];
    walk(
        &mut engine,
        |e| at(e) == At::SecondMain,
        |_, _, _| {},
        &[bear],
    );
    assert!(attacked(&engine, bear));
    walk(&mut engine, turn_is(2), |_, _, _| {}, &[]);
    assert!(
        !attacked(&engine, bear),
        "a new turn: it has not attacked this one"
    );
}
