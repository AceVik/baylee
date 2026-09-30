//! Redirection (CR 614.9): damage that would be dealt to one player or
//! creature and is dealt to another instead — a static's ("all damage that
//! would be dealt to you by unblocked creatures is dealt to this creature
//! instead") and a resolved shield's ("the next time a source of your choice
//! would deal damage to target creature this turn, that source deals that
//! damage to you instead"). Each applies once to an event (CR 614.5), after
//! the shields that prevent (the order `prevention::rank` fixes), and the
//! damage it moves is dealt, counted and triggered on where it lands.
//!
//! Played with permanents built for it, each with the sentence under test,
//! beside vanilla attackers.

use super::synthetic::{
    SyntheticLookup, creature_face, keep_mulligans, permanents, preset_both, walk_past,
};
use super::*;
use crate::event::{DamageTarget, GameEvent};
use crate::object::Status;
use crate::prevention::{Shield, ShieldKind, Shielded};
use crate::turn::Step;
use crate::zone::Zone;
use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Cost, CounterKind, Coverage, Effect, FaceDef,
    Filter, KeywordSet, Modifier, PlayerRel, TargetSpec, Trigger, activated, counters,
    static_ability, triggered,
};
use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, Defender};

// ---------------------------------------------------------------- fixtures

/// A 2/5: "As long as this creature is untapped, all damage that would be
/// dealt to you by unblocked creatures is dealt to this creature instead."
const GUARD: u32 = 1180;
/// A 0/1: "{0}: The next time a source of your choice would deal damage to
/// target creature this turn, that source deals that damage to you instead."
const MONOLITH: u32 = 1181;
/// A vanilla 2/2.
const BEAR: u32 = 1182;
/// A vanilla 3/3.
const OGRE: u32 = 1183;
/// A 1/1 with deathtouch.
const DEADLY: u32 = 1184;
/// A 4/4 with trample.
const TRAMPLER: u32 = 1185;
/// A 0/4: "Whenever you're dealt damage, put that many vitality counters
/// on this."
const WATCH: u32 = 1186;
/// A 1/1 with vigilance: "{0}: This creature deals 1 damage to any target."
const PINGER: u32 = 1187;
/// A 0/1: "For each 1 damage that would be dealt to this creature, if it
/// has a +1/+1 counter on it, remove a +1/+1 counter from it and prevent
/// that 1 damage."
const HYDRA: u32 = 1188;

static UNBLOCKED: Filter = Filter::And(&[Filter::CREATURE, Filter::Unblocked]);

static GUARDING: &[AbilityDef] = &[static_ability!(
    Filter::And(&[Filter::This, Filter::Untapped]),
    Modifier::RedirectDamageToYou(&UNBLOCKED),
)];
static SENDING: &[AbilityDef] = &[activated!(
    Cost::FREE,
    &[Effect::RedirectNextFromChosenSource {
        target: TargetSpec::Object(&Filter::CREATURE),
    }],
    target = Some(TargetSpec::Object(&Filter::CREATURE)),
)];
static WATCHING: &[AbilityDef] = &[triggered!(
    Trigger::PlayerDealtDamage(PlayerRel::You),
    &[Effect::AddCounter {
        kind: counters::VITALITY,
        amount: Amount::EventAmount,
    }],
)];
static ABSORBING: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::CountersPreventDamage(CounterKind::P1P1),
)];
static PINGING: &[AbilityDef] = &[activated!(
    Cost::FREE,
    &[Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::AnyTarget,
    }],
    target = Some(TargetSpec::AnyTarget),
)];

fn body(
    index: u32,
    name: &'static str,
    (power, toughness): (i16, i16),
    keywords: KeywordSet,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([FaceDef {
            abilities,
            keywords,
            ..creature_face(name, power, toughness, &[])
        }])),
        color_identity: ColorSet::EMPTY,
        keywords,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

fn lookup() -> SyntheticLookup {
    let none = KeywordSet::EMPTY;
    SyntheticLookup::new(vec![
        body(GUARD, "Guard", (2, 5), none, GUARDING),
        body(MONOLITH, "Monolith", (0, 1), none, SENDING),
        body(BEAR, "Bear", (2, 2), none, &[]),
        body(OGRE, "Ogre", (3, 3), none, &[]),
        body(DEADLY, "Deadly", (1, 1), KeywordSet::DEATHTOUCH, &[]),
        body(TRAMPLER, "Trampler", (4, 4), KeywordSet::TRAMPLE, &[]),
        body(WATCH, "Watch", (0, 4), none, WATCHING),
        body(PINGER, "Pinger", (1, 1), KeywordSet::VIGILANCE, PINGING),
        body(HYDRA, "Hydra", (0, 1), none, ABSORBING),
    ])
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn start(mine: &[u32], theirs: &[u32]) -> Engine<SyntheticLookup> {
    let mut engine = Engine::new(&preset_both(614, mine, theirs), lookup()).unwrap();
    keep_mulligans(&mut engine);
    engine
}

/// Walks the game until `stop` holds: in turn 1 `ME` attacks `THEM` with
/// `attackers` and `THEM` blocks with `blocks` (blocker, attacker); nothing
/// attacks or blocks after, and any other question panics.
fn walk(
    engine: &mut Engine<SyntheticLookup>,
    attackers: &[ObjectId],
    blocks: &[(ObjectId, ObjectId)],
    stop: impl Fn(&Engine<SyntheticLookup>) -> bool,
) {
    for _ in 0..400 {
        if stop(engine) {
            return;
        }
        let first = engine.state().turn.number == 1;
        match engine.pending().clone() {
            Pending::ChooseAttackers { player, .. } if player == ME && first => {
                let attackers = attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(THEM)))
                    .collect();
                engine
                    .apply(ME, PlayerAction::DeclareAttackers { attackers })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } if player == THEM && first => {
                engine
                    .apply(
                        THEM,
                        PlayerAction::DeclareBlockers {
                            blockers: blocks.to_vec(),
                        },
                    )
                    .unwrap();
            }
            other => assert!(
                walk_past(engine, &other),
                "an unexpected question: {other:?}"
            ),
        }
    }
    panic!("the game never got there");
}

fn my_end_step(engine: &Engine<SyntheticLookup>) -> bool {
    engine.state().turn.active == ME && engine.state().turn.step == Step::End
}

/// `seat` holds priority in `step` of turn 1.
fn priority_in(step: Step, seat: PlayerId) -> impl Fn(&Engine<SyntheticLookup>) -> bool {
    move |engine| {
        engine.state().turn.number == 1
            && engine.state().turn.step == step
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
    }
}

fn life(engine: &Engine<SyntheticLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

fn damage(engine: &Engine<SyntheticLookup>, id: ObjectId) -> u16 {
    engine.state().object(id).map_or(0, |o| o.damage)
}

fn zone(engine: &Engine<SyntheticLookup>, id: ObjectId) -> Option<Zone> {
    engine.state().object(id).map(|o| o.zone)
}

/// Every `DamageDealt` in the game: source, recipient, amount, combat.
fn dealt(engine: &Engine<SyntheticLookup>) -> Vec<(ObjectId, DamageTarget, u16, bool)> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::DamageDealt {
                source: Some(source),
                target,
                amount,
                is_combat,
            } => Some((source, target, amount, is_combat)),
            _ => None,
        })
        .collect()
}

/// Activates `source`'s only ability for `seat`, aimed at `object` or at
/// `player`.
fn activate(
    engine: &mut Engine<SyntheticLookup>,
    seat: PlayerId,
    source: ObjectId,
    object: Option<ObjectId>,
    player: Option<PlayerId>,
) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .expect("offered");
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: object.into_iter().collect(),
                players: player.into_iter().collect(),
            },
        )
        .expect("a legal target");
}

/// `THEM` activates the Monolith at `creature` in turn 1's declare
/// blockers step and, as it resolves, chooses `source`.
fn send(
    engine: &mut Engine<SyntheticLookup>,
    monolith: ObjectId,
    creature: ObjectId,
    source: ObjectId,
) {
    activate(engine, THEM, monolith, Some(creature), None);
    walk(engine, &[], &[], |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("walked to it")
    };
    assert_eq!(player, THEM, "the ability's controller chooses the source");
    assert!(
        options.contains(&source),
        "{source:?} is a source to choose"
    );
    engine
        .apply(
            THEM,
            PlayerAction::ChooseObjects {
                objects: vec![source],
            },
        )
        .expect("off the list");
}

// ---------------------------------------------------------------- the static

/// The Guard takes what an unblocked attacker would deal its controller:
/// the damage is dealt to it (combat damage still, from the attacker), and
/// the player is dealt none — no life lost, nothing counted for the turn,
/// no "whenever you're dealt damage". On the old engine the player took 3.
#[test]
fn an_unblocked_attackers_damage_is_dealt_to_the_untapped_guard_instead() {
    let mut engine = start(&[OGRE], &[GUARD, WATCH]);
    let ogre = permanents(&engine, OGRE)[0];
    let (guard, watch) = (permanents(&engine, GUARD)[0], permanents(&engine, WATCH)[0]);
    let before = life(&engine, THEM);

    walk(&mut engine, &[ogre], &[], my_end_step);

    assert_eq!(damage(&engine, guard), 3, "dealt to the Guard");
    assert_eq!(life(&engine, THEM), before, "and not to its controller");
    assert_eq!(engine.state().per_turn.damage_dealt_to[1], 0);
    assert_eq!(
        engine
            .state()
            .object(watch)
            .map(|o| o.counters.get(counters::VITALITY)),
        Some(0),
        "no trigger for damage the player was never dealt"
    );
    assert_eq!(
        dealt(&engine),
        vec![(ogre, DamageTarget::Object(guard), 3, true)],
        "one event of combat damage, from the Ogre to the Guard"
    );
}

/// "As long as this creature is untapped": a tapped Guard takes nothing.
/// And "unblocked creatures": an attacker that was blocked is not one, so
/// what it tramples over its blocker reaches the player, while an
/// unblocked attacker beside it is still redirected.
#[test]
fn a_tapped_guard_and_a_blocked_trampler_let_the_damage_through() {
    let mut engine = start(&[OGRE], &[GUARD]);
    let ogre = permanents(&engine, OGRE)[0];
    let guard = permanents(&engine, GUARD)[0];
    engine
        .dev_state_mut(THEM)
        .unwrap()
        .object_mut(guard)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();
    let before = life(&engine, THEM);
    walk(&mut engine, &[ogre], &[], my_end_step);
    assert_eq!(damage(&engine, guard), 0, "tapped, so not in the way");
    assert_eq!(life(&engine, THEM), before - 3);

    let mut engine = start(&[OGRE, TRAMPLER], &[GUARD, BEAR]);
    let (ogre, trampler) = (
        permanents(&engine, OGRE)[0],
        permanents(&engine, TRAMPLER)[0],
    );
    let (guard, bear) = (permanents(&engine, GUARD)[0], permanents(&engine, BEAR)[0]);
    let before = life(&engine, THEM);
    walk(
        &mut engine,
        &[ogre, trampler],
        &[(bear, trampler)],
        my_end_step,
    );
    assert_eq!(damage(&engine, guard), 3, "the unblocked Ogre's");
    assert_eq!(
        life(&engine, THEM),
        before - 2,
        "the blocked Trampler's excess over the Bear"
    );
}

/// Redirected damage is the same damage from the same source: deathtouch
/// on it destroys the Guard (CR 702.2b).
#[test]
fn deathtouch_on_the_redirected_damage_destroys_the_guard() {
    let mut engine = start(&[DEADLY], &[GUARD]);
    let deadly = permanents(&engine, DEADLY)[0];
    let guard = permanents(&engine, GUARD)[0];
    let before = life(&engine, THEM);
    walk(&mut engine, &[deadly], &[], my_end_step);
    assert_eq!(zone(&engine, guard), Some(Zone::Graveyard));
    assert_eq!(life(&engine, THEM), before);
}

/// The engine's fixed order (`prevention::rank`): a shield on the player
/// prevents first, and the Guard takes only what is left.
#[test]
fn a_shield_on_the_player_prevents_before_the_guard_takes_the_rest() {
    let mut engine = start(&[OGRE], &[GUARD]);
    let ogre = permanents(&engine, OGRE)[0];
    let guard = permanents(&engine, GUARD)[0];
    engine.dev_state_mut(THEM).unwrap().shields.push(Shield {
        protects: Shielded::Player(THEM),
        kind: ShieldKind::Next(2),
        controller: THEM,
    });
    let before = life(&engine, THEM);
    walk(&mut engine, &[ogre], &[], my_end_step);
    assert_eq!(damage(&engine, guard), 1, "3, less the 2 prevented");
    assert_eq!(life(&engine, THEM), before);
    assert!(engine.state().shields.is_empty(), "the shield is spent");
}

/// Not only combat damage: an unblocked creature's ability is redirected
/// too, through the effects' door. Before blockers are declared the same
/// creature is not yet unblocked (CR 509.1h), and its ping reaches the
/// player.
#[test]
fn an_unblocked_creatures_ability_is_redirected_but_not_before_blocks() {
    let mut engine = start(&[PINGER], &[GUARD]);
    let pinger = permanents(&engine, PINGER)[0];
    let guard = permanents(&engine, GUARD)[0];
    let before = life(&engine, THEM);

    walk(
        &mut engine,
        &[pinger],
        &[],
        priority_in(Step::DeclareAttackers, ME),
    );
    activate(&mut engine, ME, pinger, None, Some(THEM));
    walk(&mut engine, &[pinger], &[], |e| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .is_empty()
    });
    assert_eq!(
        life(&engine, THEM),
        before - 1,
        "attacking, not yet unblocked"
    );
    assert_eq!(damage(&engine, guard), 0);

    walk(
        &mut engine,
        &[pinger],
        &[],
        priority_in(Step::DeclareBlockers, ME),
    );
    activate(&mut engine, ME, pinger, None, Some(THEM));
    walk(&mut engine, &[pinger], &[], my_end_step);
    assert_eq!(
        damage(&engine, guard),
        2,
        "the ping after blocks and the combat damage"
    );
    assert_eq!(life(&engine, THEM), before - 1);
    assert!(
        dealt(&engine).contains(&(pinger, DamageTarget::Object(guard), 1, false)),
        "the ping, noncombat, to the Guard"
    );
}

// ---------------------------------------------------------------- the shield

/// The Monolith: the chosen source's next damage to the creature is dealt
/// to the Monolith's controller instead — the blocked Ogre's 3 go to the
/// player and the Bear lives — and the shield is used up. The Bear's own
/// damage to the Ogre is untouched.
#[test]
fn the_monolith_sends_the_chosen_sources_next_damage_to_its_controller() {
    let mut engine = start(&[OGRE], &[MONOLITH, BEAR]);
    let ogre = permanents(&engine, OGRE)[0];
    let (monolith, bear) = (
        permanents(&engine, MONOLITH)[0],
        permanents(&engine, BEAR)[0],
    );
    let before = life(&engine, THEM);

    walk(
        &mut engine,
        &[ogre],
        &[(bear, ogre)],
        priority_in(Step::DeclareBlockers, THEM),
    );
    send(&mut engine, monolith, bear, ogre);
    assert_eq!(engine.state().shields.len(), 1, "waiting");
    walk(&mut engine, &[ogre], &[(bear, ogre)], my_end_step);

    assert_eq!(
        life(&engine, THEM),
        before - 3,
        "the Ogre's damage, to them"
    );
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[1],
        3,
        "dealt to them"
    );
    assert_eq!(damage(&engine, bear), 0);
    assert_eq!(zone(&engine, bear), Some(Zone::Battlefield));
    assert_eq!(damage(&engine, ogre), 2, "the Bear's, where it was going");
    assert!(engine.state().shields.is_empty(), "used up");
}

/// The counter-check: another source chosen, the Ogre's damage lands on
/// the Bear, and the shield, having replaced nothing, is still waiting
/// (CR 609.7b).
#[test]
fn a_shield_for_another_source_leaves_the_damage_where_it_was() {
    let mut engine = start(&[OGRE], &[MONOLITH, BEAR]);
    let ogre = permanents(&engine, OGRE)[0];
    let (monolith, bear) = (
        permanents(&engine, MONOLITH)[0],
        permanents(&engine, BEAR)[0],
    );
    let before = life(&engine, THEM);

    walk(
        &mut engine,
        &[ogre],
        &[(bear, ogre)],
        priority_in(Step::DeclareBlockers, THEM),
    );
    send(&mut engine, monolith, bear, monolith);
    walk(&mut engine, &[ogre], &[(bear, ogre)], my_end_step);

    assert_eq!(zone(&engine, bear), Some(Zone::Graveyard), "3 on a 2/2");
    assert_eq!(life(&engine, THEM), before);
    assert_eq!(engine.state().shields.len(), 1, "not used up");
}

/// CR 614.5: the Guard takes the unblocked Ogre's damage off its
/// controller, the Monolith's shield on the Guard sends it back, and the
/// Guard, having applied once to this damage, does not take it again: the
/// player is dealt it.
#[test]
fn the_guard_does_not_take_back_what_the_monolith_sent_away() {
    let mut engine = start(&[OGRE], &[GUARD, MONOLITH]);
    let ogre = permanents(&engine, OGRE)[0];
    let (guard, monolith) = (
        permanents(&engine, GUARD)[0],
        permanents(&engine, MONOLITH)[0],
    );
    let before = life(&engine, THEM);

    walk(
        &mut engine,
        &[ogre],
        &[],
        priority_in(Step::DeclareBlockers, THEM),
    );
    send(&mut engine, monolith, guard, ogre);
    walk(&mut engine, &[ogre], &[], my_end_step);

    assert_eq!(life(&engine, THEM), before - 3);
    assert_eq!(damage(&engine, guard), 0);
    assert!(engine.state().shields.is_empty());
    assert_eq!(
        dealt(&engine),
        vec![(ogre, DamageTarget::Player(THEM), 3, true)],
        "one event, where it ended"
    );
}

/// Noncombat damage too, through the effects' door: the Monolith's shield
/// on the Bear against the Pinger sends the ping to the Monolith's
/// controller, and the Bear is dealt nothing.
#[test]
fn the_monolith_sends_a_ping_as_well() {
    let mut engine = start(&[PINGER], &[MONOLITH, BEAR]);
    let pinger = permanents(&engine, PINGER)[0];
    let (monolith, bear) = (
        permanents(&engine, MONOLITH)[0],
        permanents(&engine, BEAR)[0],
    );
    let before = life(&engine, THEM);

    walk(&mut engine, &[], &[], priority_in(Step::Upkeep, THEM));
    send(&mut engine, monolith, bear, pinger);
    walk(&mut engine, &[], &[], priority_in(Step::Main, ME));
    activate(&mut engine, ME, pinger, Some(bear), None);
    walk(&mut engine, &[], &[], my_end_step);

    assert_eq!(life(&engine, THEM), before - 1);
    assert_eq!(damage(&engine, bear), 0);
    assert_eq!(
        dealt(&engine),
        vec![(pinger, DamageTarget::Player(THEM), 1, false)]
    );
}

// ------------------------------------------------- counters that prevent

/// Puts `n` +1/+1 counters on `id`.
fn counters(engine: &mut Engine<SyntheticLookup>, seat: PlayerId, id: ObjectId, n: u16) {
    engine
        .dev_state_mut(seat)
        .unwrap()
        .object_mut(id)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, n);
    engine.refresh_offer();
}

fn p1p1(engine: &Engine<SyntheticLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .map_or(0, |o| o.counters.get(CounterKind::P1P1))
}

/// The engine's fixed order (`prevention::absorb`): a "prevent the next 1"
/// shield on the Hydra is spent before its counters, which outlast the
/// turn — the blocked Ogre's 3 cost one shield and two counters.
#[test]
fn a_shield_is_spent_before_the_hydra_s_counters() {
    let mut engine = start(&[OGRE], &[HYDRA]);
    let ogre = permanents(&engine, OGRE)[0];
    let hydra = permanents(&engine, HYDRA)[0];
    counters(&mut engine, THEM, hydra, 3);
    let version = engine.state().object(hydra).unwrap().version;
    engine.dev_state_mut(THEM).unwrap().shields.push(Shield {
        protects: Shielded::Object(hydra, version),
        kind: ShieldKind::Next(1),
        controller: THEM,
    });

    walk(&mut engine, &[ogre], &[(hydra, ogre)], my_end_step);

    assert!(engine.state().shields.is_empty(), "the shield went first");
    assert_eq!(p1p1(&engine, hydra), 1, "then two counters");
    assert_eq!(damage(&engine, hydra), 0, "and nothing was dealt");
}

/// Damage the Monolith moves off the Hydra is never dealt to it, so it
/// costs no counter: the redirection is asked first.
#[test]
fn damage_the_monolith_moves_costs_the_hydra_no_counter() {
    let mut engine = start(&[OGRE], &[HYDRA, MONOLITH]);
    let ogre = permanents(&engine, OGRE)[0];
    let (hydra, monolith) = (
        permanents(&engine, HYDRA)[0],
        permanents(&engine, MONOLITH)[0],
    );
    counters(&mut engine, THEM, hydra, 3);
    let before = life(&engine, THEM);

    walk(
        &mut engine,
        &[ogre],
        &[(hydra, ogre)],
        priority_in(Step::DeclareBlockers, THEM),
    );
    send(&mut engine, monolith, hydra, ogre);
    walk(&mut engine, &[ogre], &[(hydra, ogre)], my_end_step);

    assert_eq!(life(&engine, THEM), before - 3, "moved to its controller");
    assert_eq!(p1p1(&engine, hydra), 3, "no counter spent");
}

/// Noncombat damage too, through the effects' door: a ping takes a
/// counter and marks nothing; with no counter left the next one is dealt.
#[test]
fn a_ping_takes_a_counter_and_then_is_dealt() {
    let mut engine = start(&[PINGER], &[HYDRA]);
    let pinger = permanents(&engine, PINGER)[0];
    let hydra = permanents(&engine, HYDRA)[0];
    counters(&mut engine, THEM, hydra, 1);

    walk(&mut engine, &[], &[], priority_in(Step::Main, ME));
    activate(&mut engine, ME, pinger, Some(hydra), None);
    walk(&mut engine, &[], &[], |e| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .is_empty()
    });
    assert_eq!(p1p1(&engine, hydra), 0, "the counter paid for it");
    assert_eq!(damage(&engine, hydra), 0);
    assert_eq!(zone(&engine, hydra), Some(Zone::Battlefield));

    activate(&mut engine, ME, pinger, Some(hydra), None);
    walk(&mut engine, &[], &[], |e| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .is_empty()
    });
    assert_eq!(zone(&engine, hydra), Some(Zone::Graveyard), "1 on a 0/1");
}
