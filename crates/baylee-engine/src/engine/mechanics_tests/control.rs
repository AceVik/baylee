//! A change of control, and the two clocks a permanent is read against
//! (#290).
//!
//! CR 613.7 lists every event that gives an object a new timestamp: entering
//! a zone (613.7d), becoming attached (613.7e), turning face up or down
//! (613.7f), transforming or converting (613.7g). A change of control is not
//! among them, so the effects of a stolen permanent's static abilities keep
//! the timestamp they had (613.7a), and so does the permanent wherever its
//! timestamp orders anything else. What a change of control does restart is
//! the other clock: CR 302.6 asks whether a creature has been under its
//! controller's control continuously since their most recent turn began.
//! One field used to hold both, so every change of control restamped the
//! permanent.
//!
//! The cards are nobody's: a creature whose static ability sets its own base
//! power and toughness to 5/5 (layer 7b, CR 613.4b), with "{0}: this creature
//! has base power and toughness 2/2" and an end-step trigger; a thief with
//! "{0}: gain control of target creature"; and a bystander with the same
//! end-step trigger as the creature.

use super::*;
use baylee_cards_dsl::{Duration, Filter, Modifier, PlayerRel, StepKind, TargetSpec, Trigger};

const LOYAL: u32 = 7320;
const THIEF: u32 = 7321;
const BYSTANDER: u32 = 7322;

static GAIN_ONE: &[Effect] = &[Effect::gain_life(1)];
static BECOME_TWO: &[Effect] = &[Effect::continuous(
    &Filter::This,
    Modifier::SetPT(2, 2),
    Duration::Indefinitely,
)];
static TAKE: &[Effect] = &[Effect::continuous(
    &Filter::This,
    Modifier::GainControl,
    Duration::Indefinitely,
)];
static ANY_CREATURE: Filter = Filter::CREATURE;

/// "At the beginning of your end step, you gain 1 life."
const AT_YOUR_END_STEP: AbilityDef = baylee_cards_dsl::triggered!(
    Trigger::StepBegin {
        step: StepKind::End,
        whose: PlayerRel::You,
    },
    GAIN_ONE
);

/// 0: "This creature has base power and toughness 5/5", whose effect has the
/// permanent's timestamp (CR 613.7a); 1: "{0}: … 2/2"; 2: the end-step
/// trigger.
static LOYAL_ABILITIES: &[AbilityDef] = &[
    baylee_cards_dsl::static_ability!(Filter::This, Modifier::SetPT(5, 5)),
    free(BECOME_TWO, None),
    AT_YOUR_END_STEP,
];
/// 0: "{0}: gain control of target creature."
static THIEF_ABILITIES: &[AbilityDef] = &[free(
    TAKE,
    Some(TargetReq::one(TargetSpec::Object(&ANY_CREATURE))),
)];
static BYSTANDER_ABILITIES: &[AbilityDef] = &[AT_YOUR_END_STEP];

fn cards() -> Vec<&'static CardDef> {
    vec![
        card(
            LOYAL,
            creature_face("Loyal", "{0}", 3, 3),
            KeywordSet::EMPTY,
            LOYAL_ABILITIES,
        ),
        card(
            THIEF,
            creature_face("Thief", "{0}", 1, 1),
            KeywordSet::EMPTY,
            THIEF_ABILITIES,
        ),
        card(
            BYSTANDER,
            creature_face("Bystander", "{0}", 1, 1),
            KeywordSet::EMPTY,
            BYSTANDER_ABILITIES,
        ),
    ]
}

/// The permanent's projected power and toughness.
fn pt(engine: &Bench, id: ObjectId) -> (Option<i16>, Option<i16>) {
    let chars = engine
        .state()
        .object(id)
        .expect("on the battlefield")
        .characteristics();
    (chars.power, chars.toughness)
}

/// Seat 0's Loyal, made 2/2 on seat 0's turn, then taken by seat 1's Thief
/// in seat 1's first main phase. Seat 1 holds priority on an empty stack.
fn stolen_after_the_two(seed: u64) -> (Bench, ObjectId) {
    let mut engine = bench(
        seed,
        cards(),
        [Seat::with(&[LOYAL]), Seat::with(&[THIEF, BYSTANDER])],
    );
    to_main(&mut engine, me());
    let loyal = the(&engine, ZoneLocation::Battlefield, LOYAL);
    assert_eq!(pt(&engine, loyal), (Some(5), Some(5)), "the static's 5/5");
    activate(&mut engine, me(), loyal, 1);
    settle(&mut engine, me());
    assert_eq!(
        pt(&engine, loyal),
        (Some(2), Some(2)),
        "the 2/2 is newer than the static's 5/5 and applies after it"
    );

    to_main(&mut engine, them());
    let thief = the(&engine, ZoneLocation::Battlefield, THIEF);
    activate(&mut engine, them(), thief, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the theft targets: {:?}", engine.pending())
    };
    assert!(options.contains(&loyal), "Loyal is a creature: {options:?}");
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![loyal],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, them());
    assert_eq!(
        engine.state().object(loyal).map(|o| o.controller),
        Some(them()),
        "the Thief took it"
    );
    (engine, loyal)
}

/// CR 613.7a, read in the one place a timestamp shows: which of two
/// layer-7b effects applies last. The 2/2 was made after Loyal entered, so
/// it applies after Loyal's own 5/5, and a change of control is not an event
/// CR 613.7 restamps a permanent for. Restamped, the static's 5/5 would sort
/// after the 2/2 and win.
#[test]
fn a_stolen_permanents_static_keeps_its_place_in_the_layer() {
    let (engine, loyal) = stolen_after_the_two(7320);
    assert_eq!(
        pt(&engine, loyal),
        (Some(2), Some(2)),
        "the static keeps Loyal's timestamp, older than the 2/2"
    );
}

/// The other clock does restart (CR 302.6): seat 1 has controlled Loyal
/// only since the middle of its own turn, so Loyal may not attack for it.
/// Asked through the offer, where a player meets it.
#[test]
fn a_stolen_creature_is_summoning_sick_for_its_taker() {
    let (mut engine, loyal) = stolen_after_the_two(7321);
    let thief = the(&engine, ZoneLocation::Battlefield, THIEF);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
            || !matches!(e.state().turn.phase, Phase::FirstMain | Phase::Combat)
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!(
            "the turn went past combat without asking for attackers: {:?}",
            engine.pending()
        )
    };
    assert!(
        attackers.contains(&thief),
        "the Thief has been seat 1's since the turn began: {attackers:?}"
    );
    assert!(
        !attackers.contains(&loyal),
        "Loyal came under seat 1's control this turn: {attackers:?}"
    );
}

/// The timestamp orders more than layers: two triggered abilities of one
/// controller that trigger together are put on the stack oldest first here
/// (CR 603.3b lets the controller choose; the engine does not ask). Loyal
/// entered before the Bystander, and taking it does not make it newer, so
/// at seat 1's end step Loyal's trigger still goes on the stack first.
#[test]
fn a_change_of_control_leaves_the_trigger_order_as_it_was() {
    let (mut engine, loyal) = stolen_after_the_two(7322);
    let bystander = the(&engine, ZoneLocation::Battlefield, BYSTANDER);
    assert!(
        engine.state().object(loyal).map(|o| o.timestamp)
            < engine.state().object(bystander).map(|o| o.timestamp),
        "Loyal entered first"
    );
    let from = engine.state().journal.len();
    walk_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End && !e.state().zones.stack_is_empty()
    });
    let order: Vec<ObjectId> = engine.state().journal.entries()[from..]
        .iter()
        .filter_map(|entry| match entry.event {
            crate::event::GameEvent::AbilityTriggered { source, .. }
                if source == loyal || source == bystander =>
            {
                Some(source)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        order,
        vec![loyal, bystander],
        "both trigger for seat 1, Loyal's first"
    );
}
