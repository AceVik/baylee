//! Existing shield predicates exercised through the shared damage procedure.

use super::*;
use crate::object::ObjectKind;
use crate::prevention::{ChosenSource, Shield, Shielded};
use crate::state::CardLookup;
use crate::zone::Zone;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::Filter;
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};
use baylee_core::types::TypeSet;

struct NoCards;
impl CardLookup for NoCards {
    fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        None
    }
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn board() -> (GameState, ObjectId) {
    let seat = || SeatSpec {
        controller: SeatController::Open,
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: vec![],
        sideboard: vec![],
        commanders: vec![],
        starting_life: Some(20),
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let mut state = GameState::from_preset(
        &GamePreset {
            format: FormatId::Freeform,
            seed: 1,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![],
            seats: vec![seat(), seat()],
        },
        &NoCards,
    )
    .expect("an empty board");
    let name = state.names.intern("Test Creature");
    let id = state.create_bare(P0, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    let b = state.object_mut(id).expect("just created").base_mut();
    b.types = TypeSet::CREATURE;
    b.power = Some(2);
    b.toughness = Some(2);
    (state, id)
}

fn shield(protects: Shielded, kind: ShieldKind) -> Shield {
    Shield {
        protects,
        kind,
        controller: P0,
    }
}

/// CR 615.7: "the next 3" prevents 3 in all, across events, and only
/// to what it shields.
#[test]
fn a_shield_prevents_its_amount_and_is_used_up() {
    let (mut state, creature) = board();
    state
        .shields
        .push(shield(Shielded::Player(P1), ShieldKind::Next(3)));
    assert_eq!(
        apply(&mut state, creature, DamageTarget::Player(P0), 2, false),
        2
    );
    assert_eq!(
        apply(&mut state, creature, DamageTarget::Player(P1), 2, false),
        0
    );
    assert_eq!(state.shields[0].kind, ShieldKind::Next(1));
    assert_eq!(
        apply(&mut state, creature, DamageTarget::Player(P1), 2, true),
        1
    );
    assert!(state.shields.is_empty(), "used up");
}

/// A shield on a permanent is on that object: once it has left and
/// come back, the new object is not shielded (CR 400.7).
#[test]
fn a_shield_on_a_permanent_is_on_that_object() {
    let (mut state, creature) = board();
    let version = state.object(creature).unwrap().version;
    state.shields.push(shield(
        Shielded::Object(creature, version),
        ShieldKind::Next(1),
    ));
    state.object_mut(creature).unwrap().version += 1;
    assert_eq!(
        apply(
            &mut state,
            creature,
            DamageTarget::Object(creature),
            1,
            false
        ),
        1
    );
    assert_eq!(state.shields.len(), 1, "not used on a different object");
}

/// Choosing Fog first prevents combat damage and preserves a "next 2"
/// beside it is still whole afterwards.
#[test]
fn fog_prevents_combat_damage_and_spares_the_other_shields() {
    let (mut state, creature) = board();
    state
        .shields
        .push(shield(Shielded::Player(P1), ShieldKind::Next(2)));
    state
        .shields
        .push(shield(Shielded::Everything, ShieldKind::AllCombat));
    assert_eq!(
        apply_selected(
            &mut state,
            creature,
            DamageTarget::Player(P1),
            5,
            true,
            |k| matches!(k, DamageEffectKind::PreventCombat)
        ),
        0
    );
    assert_eq!(state.shields[0].kind, ShieldKind::Next(2));
    assert_eq!(
        apply(&mut state, creature, DamageTarget::Player(P1), 5, false),
        3
    );
    assert_eq!(state.shields.len(), 1, "the amount is spent, Fog stays");
}

static RED: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Red]));

fn painted(state: &mut GameState, id: ObjectId, colors: ColorSet) {
    state
        .object_mut(id)
        .expect("on the board")
        .base_mut()
        .colors = colors;
    state.refresh_characteristics();
}

/// A red creature beside `board()`'s, for a source that was not chosen.
fn another(state: &mut GameState) -> ObjectId {
    let name = state.names.intern("Another Creature");
    let id = state.create_bare(P0, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    state.object_mut(id).expect("just created").base_mut().types = TypeSet::CREATURE;
    painted(state, id, ColorSet::from_slice(&[Color::Red]));
    id
}

/// P1's shield against `source`, as resolving the ability would leave it.
fn shield_against(
    state: &mut GameState,
    source: ObjectId,
    filter: &'static Filter,
    all_but: u32,
    gain_life: bool,
    combat_only: bool,
) {
    let chosen = ChosenSource::new(state, source, filter, P1, source).expect("on the board");
    state.shields.push(Shield {
        protects: Shielded::Player(P1),
        kind: ShieldKind::NextFrom {
            source: chosen,
            all_but,
            gain_life,
            combat_only,
        },
        controller: P1,
    });
}

fn moved(state: &mut GameState, id: ObjectId, to: ZoneLocation) {
    state
        .move_object(
            id,
            to,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::StateBased,
        )
        .expect("it moves");
}

/// CR 615.8 and 609.7b: the next instance from the chosen source, all
/// of it — and only while the source is still what it was chosen as.
/// Damage it does not prevent does not use it up.
#[test]
fn a_chosen_source_shield_waits_for_its_source_and_rechecks_it() {
    let (mut state, chosen) = board();
    painted(&mut state, chosen, ColorSet::from_slice(&[Color::Red]));
    let other = another(&mut state);
    shield_against(&mut state, chosen, &RED, 0, false, false);

    let to_p1 = DamageTarget::Player(P1);
    assert_eq!(
        apply(&mut state, other, to_p1, 3, false),
        3,
        "another source"
    );
    assert_eq!(
        apply(&mut state, chosen, DamageTarget::Player(P0), 3, false),
        3,
        "another player"
    );
    painted(&mut state, chosen, ColorSet::from_slice(&[Color::White]));
    assert_eq!(
        apply(&mut state, chosen, to_p1, 3, false),
        3,
        "no longer red, so the shield does not apply (CR 609.7b)"
    );
    assert_eq!(state.shields.len(), 1, "and is not used up");

    painted(&mut state, chosen, ColorSet::from_slice(&[Color::Red]));
    assert_eq!(apply(&mut state, chosen, to_p1, 7, true), 0, "all of it");
    assert!(state.shields.is_empty(), "one instance, then gone");
    assert_eq!(apply(&mut state, chosen, to_p1, 2, false), 2);
}

/// The pinger killed in response: its ability still deals the damage,
/// from the source it was, and the shield on that source still applies
/// (CR 609.7a) — rechecked against what it last was. The card that came
/// back is a new object (CR 400.7), and not the source chosen.
#[test]
fn a_chosen_source_that_has_left_is_still_the_source_it_was() {
    let to_p1 = DamageTarget::Player(P1);

    let (mut state, pinger) = board();
    painted(&mut state, pinger, ColorSet::from_slice(&[Color::Red]));
    shield_against(&mut state, pinger, &RED, 0, false, false);
    moved(&mut state, pinger, ZoneLocation::Graveyard(P0));
    assert_eq!(apply(&mut state, pinger, to_p1, 2, false), 0);
    assert!(state.shields.is_empty());

    let (mut state, pinger) = board();
    painted(&mut state, pinger, ColorSet::from_slice(&[Color::Red]));
    shield_against(&mut state, pinger, &RED, 0, false, false);
    painted(&mut state, pinger, ColorSet::from_slice(&[Color::White]));
    moved(&mut state, pinger, ZoneLocation::Graveyard(P0));
    assert_eq!(
        apply(&mut state, pinger, to_p1, 2, false),
        2,
        "it left white, and that is what it last was"
    );

    let (mut state, pinger) = board();
    painted(&mut state, pinger, ColorSet::from_slice(&[Color::Red]));
    shield_against(&mut state, pinger, &RED, 0, false, false);
    moved(&mut state, pinger, ZoneLocation::Graveyard(P0));
    moved(&mut state, pinger, ZoneLocation::Battlefield);
    painted(&mut state, pinger, ColorSet::from_slice(&[Color::Red]));
    assert_eq!(
        apply(&mut state, pinger, to_p1, 2, false),
        2,
        "the creature that came back is another object"
    );
    assert_eq!(state.shields.len(), 1);
}

/// Forcefield: combat damage only, all but 1 of it, and an instance of
/// 1 is one it prevents nothing of — so it is still there after it.
#[test]
fn all_but_one_of_combat_damage_and_nothing_else() {
    let (mut state, attacker) = board();
    shield_against(&mut state, attacker, &Filter::Any, 1, false, true);
    let to_p1 = DamageTarget::Player(P1);
    assert_eq!(
        apply(&mut state, attacker, to_p1, 3, false),
        3,
        "not combat"
    );
    assert_eq!(apply(&mut state, attacker, to_p1, 1, true), 1);
    assert_eq!(state.shields.len(), 1, "it prevented nothing (CR 609.7b)");
    assert_eq!(apply(&mut state, attacker, to_p1, 4, true), 1);
    assert!(state.shields.is_empty());
}

/// Choosing life prevention before Fog gains life and consumes that shield.
#[test]
fn the_player_can_choose_life_prevention_before_fog() {
    let (mut state, source) = board();
    let life = state.players[1].life;
    state
        .shields
        .push(shield(Shielded::Everything, ShieldKind::AllCombat));
    shield_against(&mut state, source, &Filter::Any, 0, true, false);
    assert_eq!(
        apply_selected(
            &mut state,
            source,
            DamageTarget::Player(P1),
            3,
            true,
            |k| matches!(
                k,
                DamageEffectKind::PreventFromSource {
                    gain_life: true,
                    ..
                }
            )
        ),
        0
    );
    assert_eq!(state.players[1].life, life + 3, "paid back");
    assert_eq!(state.shields.len(), 1);
    assert_eq!(state.shields[0].kind, ShieldKind::AllCombat, "Fog stays");
}

/// Choosing Fog before a source shield preserves it; choosing the full
/// source shield before an all-but-one shield preserves the latter.
#[test]
fn choosing_the_fuller_shield_keeps_the_other() {
    let (mut state, source) = board();
    let to_p1 = DamageTarget::Player(P1);
    shield_against(&mut state, source, &Filter::Any, 0, false, false);
    state
        .shields
        .push(shield(Shielded::Everything, ShieldKind::AllCombat));
    assert_eq!(
        apply_selected(&mut state, source, to_p1, 3, true, |k| matches!(
            k,
            DamageEffectKind::PreventCombat
        )),
        0
    );
    assert_eq!(
        state.shields.len(),
        2,
        "Fog prevented it, the Circle stands"
    );

    let (mut state, source) = board();
    shield_against(&mut state, source, &Filter::Any, 1, false, true);
    shield_against(&mut state, source, &Filter::Any, 0, false, false);
    assert_eq!(
        apply_selected(&mut state, source, to_p1, 3, true, |k| matches!(
            k,
            DamageEffectKind::PreventFromSource { all_but: 0, .. }
        )),
        0
    );
    assert_eq!(state.shields.len(), 1);
    assert!(
        matches!(
            state.shields[0].kind,
            ShieldKind::NextFrom { all_but: 1, .. }
        ),
        "Forcefield stands"
    );
}

/// Jade Monolith's shield on P0's creature, sending `source`'s next
/// damage to P1.
fn redirect_shield(state: &mut GameState, creature: ObjectId, source: ObjectId) {
    let version = state.object(creature).expect("on the board").version;
    let chosen = ChosenSource::new(state, source, &Filter::Any, P1, source).expect("there");
    state.shields.push(Shield {
        protects: Shielded::Object(creature, version),
        kind: ShieldKind::RedirectNextFrom {
            source: chosen,
            to: P1,
        },
        controller: P1,
    });
}

/// CR 614.9 with 609.7b: the chosen source's next damage to the
/// creature goes to the player instead, all of it, and the shield is
/// used up; damage from another source, or to anything else, leaves it
/// waiting. It prevents nothing, so `apply` passes it by.
#[test]
fn a_redirection_shield_moves_its_sources_next_damage_once() {
    let (mut state, creature) = board();
    let source = another(&mut state);
    redirect_shield(&mut state, creature, source);
    let to_it = DamageTarget::Object(creature);

    assert_eq!(
        apply(&mut state, source, to_it, 3, false),
        3,
        "not prevention"
    );
    let mut done = Redirected::default();
    assert_eq!(
        redirect(&mut state, creature, to_it, &mut done),
        None,
        "another source"
    );
    assert_eq!(
        redirect(&mut state, source, DamageTarget::Object(source), &mut done),
        None,
        "another creature"
    );
    assert_eq!(
        redirect(&mut state, source, DamageTarget::Player(P0), &mut done),
        None,
        "a player"
    );
    assert_eq!(state.shields.len(), 1, "still waiting (CR 609.7b)");

    assert_eq!(
        redirect(&mut state, source, to_it, &mut done),
        Some(DamageTarget::Player(P1))
    );
    assert!(state.shields.is_empty(), "used up");
    assert_eq!(redirect(&mut state, source, to_it, &mut done), None);
}

/// CR 614.9: to a player who has left the game, a redirection does
/// nothing, and so replaces nothing and is not used up — nor from a
/// permanent that is no longer a creature.
#[test]
fn a_redirection_to_a_player_who_has_left_does_nothing() {
    let (mut state, creature) = board();
    let source = another(&mut state);
    redirect_shield(&mut state, creature, source);
    state.players[1].loss = Some(crate::event::LossReason::Conceded);
    assert_eq!(
        redirect(
            &mut state,
            source,
            DamageTarget::Object(creature),
            &mut Redirected::default()
        ),
        None
    );
    assert_eq!(state.shields.len(), 1);

    let (mut state, creature) = board();
    let source = another(&mut state);
    redirect_shield(&mut state, creature, source);
    state
        .object_mut(creature)
        .expect("on the board")
        .base_mut()
        .types = TypeSet::ARTIFACT;
    state.refresh_characteristics();
    assert_eq!(
        redirect(
            &mut state,
            source,
            DamageTarget::Object(creature),
            &mut Redirected::default()
        ),
        None,
        "no longer a creature"
    );
    assert_eq!(state.shields.len(), 1);
}

/// Veteran Bodyguard's static, on `bodyguard`, for P0: red sources'
/// damage to P0 is dealt to it instead.
fn bodyguard(state: &mut GameState, bodyguard: ObjectId) {
    let modifier = baylee_cards_dsl::Modifier::RedirectDamageToYou(&RED);
    state.effects.register(crate::effects::ContinuousEffect {
        // `register` assigns the real one.
        id: baylee_core::ids::EffectId::new(0),
        source: Some(bodyguard),
        controller: P0,
        origin: crate::effects::EffectOrigin::Static,
        layer: modifier.layer(),
        timestamp: 1,
        duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
        filter: crate::effects::EffectFilter::Dsl(&Filter::This),
        modifier,
    });
}

/// A redirecting static: damage to its controller from a matching
/// source goes to the permanent, and never twice in one event
/// (CR 614.5) — a new event is another matter. Not another player's
/// damage, not a source that does not match, and nothing once the
/// permanent is no longer a creature (CR 614.9).
#[test]
fn a_redirecting_static_moves_its_controllers_damage_once_an_event() {
    let (mut state, guard) = board();
    let red = another(&mut state);
    bodyguard(&mut state, guard);
    let to_p0 = DamageTarget::Player(P0);
    let mut done = Redirected::default();

    assert_eq!(
        redirect(&mut state, red, DamageTarget::Player(P1), &mut done),
        None,
        "its controller's damage only"
    );
    assert_eq!(
        redirect(&mut state, guard, to_p0, &mut done),
        None,
        "not red"
    );
    assert_eq!(
        redirect(&mut state, red, to_p0, &mut done),
        Some(DamageTarget::Object(guard))
    );
    assert_eq!(
        redirect(&mut state, red, to_p0, &mut done),
        None,
        "once to an event (CR 614.5)"
    );
    assert_eq!(
        redirect(&mut state, red, to_p0, &mut Redirected::default()),
        Some(DamageTarget::Object(guard)),
        "and again to the next one"
    );

    state
        .object_mut(guard)
        .expect("on the board")
        .base_mut()
        .types = TypeSet::ARTIFACT;
    state.refresh_characteristics();
    assert_eq!(
        redirect(&mut state, red, to_p0, &mut Redirected::default()),
        None,
        "no longer a creature (CR 614.9)"
    );

    let (mut state, guard) = board();
    let red = another(&mut state);
    bodyguard(&mut state, guard);
    state.players[0].loss = Some(crate::event::LossReason::Conceded);
    assert_eq!(
        redirect(&mut state, red, to_p0, &mut Redirected::default()),
        None,
        "from a player who has left the game (CR 614.9)"
    );
}

/// CR 609.7c: a source that has left the battlefield is asked as it
/// last was there — the ability of a red creature killed in response is
/// a red source's; one that left white is not.
#[test]
fn a_redirecting_static_reads_a_departed_source_as_it_last_was() {
    let to_p0 = DamageTarget::Player(P0);

    let (mut state, guard) = board();
    let red = another(&mut state);
    bodyguard(&mut state, guard);
    moved(&mut state, red, ZoneLocation::Graveyard(P0));
    // The card it is now is not what it was: only the last known
    // characteristics say red.
    painted(&mut state, red, ColorSet::from_slice(&[Color::White]));
    assert_eq!(
        redirect(&mut state, red, to_p0, &mut Redirected::default()),
        Some(DamageTarget::Object(guard)),
        "it left red"
    );

    let (mut state, guard) = board();
    let red = another(&mut state);
    bodyguard(&mut state, guard);
    painted(&mut state, red, ColorSet::from_slice(&[Color::White]));
    moved(&mut state, red, ZoneLocation::Graveyard(P0));
    painted(&mut state, red, ColorSet::from_slice(&[Color::Red]));
    assert_eq!(
        redirect(&mut state, red, to_p0, &mut Redirected::default()),
        None,
        "it left white"
    );
}

/// Rock Hydra's static on `hydra`, with `n` +1/+1 counters on it.
fn hydra(state: &mut GameState, hydra: ObjectId, n: u16) {
    let modifier =
        baylee_cards_dsl::Modifier::CountersPreventDamage(baylee_cards_dsl::CounterKind::P1P1);
    state.effects.register(crate::effects::ContinuousEffect {
        // `register` assigns the real one.
        id: baylee_core::ids::EffectId::new(0),
        source: Some(hydra),
        controller: P0,
        origin: crate::effects::EffectOrigin::Static,
        layer: modifier.layer(),
        timestamp: 1,
        duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
        filter: crate::effects::EffectFilter::Dsl(&Filter::This),
        modifier,
    });
    state
        .object_mut(hydra)
        .expect("on the board")
        .counters
        .set(baylee_cards_dsl::CounterKind::P1P1, n);
}

fn p1p1(state: &GameState, id: ObjectId) -> u16 {
    state
        .object(id)
        .map_or(0, |o| o.counters.get(baylee_cards_dsl::CounterKind::P1P1))
}

/// Rock Hydra: each 1 damage takes a counter and is prevented while a
/// counter is there; the rest is dealt. With none left nothing is
/// removed, and another permanent is not covered.
#[test]
fn counters_prevent_damage_one_for_one() {
    let (mut state, it) = board();
    let other = another(&mut state);
    hydra(&mut state, it, 3);

    assert_eq!(absorb(&mut state, other, it, 2, false), 0);
    assert_eq!(p1p1(&state, it), 1);
    assert_eq!(
        absorb(&mut state, other, it, 3, true),
        2,
        "one counter, one prevented"
    );
    assert_eq!(p1p1(&state, it), 0);
    let entries = state.journal.len();
    assert_eq!(absorb(&mut state, other, it, 2, false), 2);
    assert_eq!(
        state.journal.len(),
        entries,
        "nothing removed, nothing journalled"
    );

    state
        .object_mut(other)
        .expect("on the board")
        .counters
        .set(baylee_cards_dsl::CounterKind::P1P1, 2);
    assert_eq!(absorb(&mut state, it, other, 2, false), 2, "not the Hydra");
    assert_eq!(p1p1(&state, other), 2);
}

/// CR 615.12: damage that can't be prevented is still offered to the
/// effect, which prevents none of it and removes the counters anyway,
/// once for the event (615.12a).
#[test]
fn unpreventable_damage_still_takes_the_counters() {
    let (mut state, it) = board();
    let source = another(&mut state);
    hydra(&mut state, it, 3);
    let modifier = baylee_cards_dsl::Modifier::CombatDamageCantBePrevented;
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp: 2,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter: crate::effects::EffectFilter::Dsl(&Filter::Any),
        modifier,
    });

    assert_eq!(
        absorb(&mut state, source, it, 2, true),
        2,
        "all of it dealt"
    );
    assert_eq!(p1p1(&state, it), 1, "and two counters gone");
    assert_eq!(
        absorb(&mut state, source, it, 2, false),
        1,
        "noncombat damage can still be prevented"
    );
    assert_eq!(p1p1(&state, it), 0);
}

// These focused predicate tests deliberately stop before damage results and
// exercise the production candidate/application functions, never a second
// prevention implementation. Competing effects require an explicit choice.
fn work(
    state: &mut GameState,
    source: ObjectId,
    recipient: DamageTarget,
    amount: u32,
    is_combat: bool,
) -> DamageWork {
    state.refresh_characteristics();
    let version = state.object(source).and_then(|obj| {
        if matches!(obj.zone, Zone::Battlefield | Zone::Stack) {
            Some(obj.version)
        } else {
            state
                .ltb_versions
                .iter()
                .find(|(id, _)| *id == source)
                .map(|(_, v)| *v)
        }
    });
    DamageWork::new(
        state,
        vec![Assignment {
            source,
            source_version: version,
            recipient,
            amount,
            is_combat,
        }],
    )
}
fn apply(
    state: &mut GameState,
    source: ObjectId,
    recipient: DamageTarget,
    amount: u32,
    is_combat: bool,
) -> u32 {
    apply_selected(state, source, recipient, amount, is_combat, |_| {
        panic!("fixture must choose a competing effect")
    })
}
fn apply_selected(
    state: &mut GameState,
    source: ObjectId,
    recipient: DamageTarget,
    amount: u32,
    is_combat: bool,
    choose: impl Fn(DamageEffectKind) -> bool,
) -> u32 {
    let mut work = work(state, source, recipient, amount, is_combat);
    loop {
        let mut options = work.candidates(state);
        options.retain(|c| {
            !matches!(
                c.option.kind,
                DamageEffectKind::Redirect { .. } | DamageEffectKind::RemoveCounter { .. }
            )
        });
        if options.is_empty() {
            work.commit_additional(state);
            return work.dealt();
        }
        let candidate = if options.len() == 1 {
            options.remove(0)
        } else {
            options
                .into_iter()
                .find(|c| choose(c.option.kind))
                .expect("chosen legal effect")
        };
        assert!(work.select(state, candidate, false).is_none());
    }
}
#[derive(Default)]
struct Redirected {
    applied: Vec<EffectKey>,
}
fn redirect(
    state: &mut GameState,
    source: ObjectId,
    recipient: DamageTarget,
    done: &mut Redirected,
) -> Option<DamageTarget> {
    let mut work = work(state, source, recipient, 1, false);
    work.parts[0].applied = done.applied.clone();
    let mut options = work.candidates(state);
    options.retain(|c| matches!(c.option.kind, DamageEffectKind::Redirect { .. }));
    if options.is_empty() {
        return None;
    }
    assert_eq!(options.len(), 1, "fixture must choose among redirections");
    work.apply_effect(state, &options[0]);
    done.applied = work.parts[0].applied.clone();
    Some(work.parts[0].view.recipient)
}
fn absorb(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: u32,
    is_combat: bool,
) -> u32 {
    let mut work = work(
        state,
        source,
        DamageTarget::Object(target),
        amount,
        is_combat,
    );
    loop {
        let mut options = work.candidates(state);
        options.retain(|c| matches!(c.option.kind, DamageEffectKind::RemoveCounter { .. }));
        if options.is_empty() {
            work.commit_additional(state);
            return work.dealt();
        }
        assert_eq!(options.len(), 1);
        assert!(work.select(state, options.remove(0), true).is_none());
    }
}
