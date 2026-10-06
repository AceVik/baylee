use super::*;
use crate::effects::EffectFilter;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_cards_dsl::{Filter, Modifier, ZoneRef};

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn them() -> PlayerId {
    PlayerId::new(1)
}

fn state() -> GameState {
    GameState::from_preset(&preset(11, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game")
}

/// A permanent on the battlefield with a controller and nothing else.
///
/// Bare because what `bound_now` reads is the battlefield list and the
/// controller, and a printed card would put four other characteristics
/// in front of the one question being asked.
fn permanent(state: &mut GameState, seat: PlayerId, name: &str) -> ObjectId {
    let name = state.names.intern(name);
    state.create_bare(seat, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
}

/// The ids a bound set names, in the order it named them.
fn named(bound: &[EffectFilter]) -> Vec<ObjectId> {
    bound
        .iter()
        .map(|f| match f {
            EffectFilter::ObjectIs(id, _) => *id,
            EffectFilter::Dsl(filter) => panic!("a dynamic filter in a bound set: {filter:?}"),
        })
        .collect()
}

/// CR 611.2c: an effect from a *resolution* that modifies
/// characteristics affects the objects it found and no others. So the
/// filter is read here, once, and the effect is registered against each
/// object it named — one `EffectFilter` per object, because an
/// `EffectFilter` names exactly one.
///
/// The permanent created afterwards is the whole point: the same call
/// made a moment later names it, and the set already bound does not.
#[test]
fn a_locking_modifier_binds_the_board_it_found() {
    let mut state = state();
    let first = permanent(&mut state, me(), "First");
    let second = permanent(&mut state, me(), "Second");

    let bound = bound_now(
        &state,
        &Filter::Any,
        &Modifier::ModifyPT(-2, -2),
        me(),
        first,
        None,
    );
    assert_eq!(
        named(&bound),
        vec![first, second],
        "one per permanent, in battlefield order"
    );

    let latecomer = permanent(&mut state, me(), "Latecomer");
    assert!(
        !named(&bound).contains(&latecomer),
        "\"all creatures get -2/-2 until end of turn\" leaves a creature \
         that arrives afterwards alone"
    );
    let again = bound_now(
        &state,
        &Filter::Any,
        &Modifier::ModifyPT(-2, -2),
        me(),
        first,
        None,
    );
    assert_eq!(
        named(&again),
        vec![first, second, latecomer],
        "and it is the moment that bound the set, not the filter"
    );
}

/// The set is bound by **identity** and not by id, which is the pair
/// `EffectFilter::object` exists to write: a permanent that leaves and
/// comes back keeps its id and is a new object (CR 400.7), so the
/// version the set recorded is the one it was bound at.
#[test]
fn a_bound_set_records_the_version_it_saw() {
    let mut state = state();
    let bear = permanent(&mut state, me(), "Bear");
    let bound = bound_now(
        &state,
        &Filter::Any,
        &Modifier::ModifyPT(1, 1),
        me(),
        bear,
        None,
    );
    let object = state.object(bear).expect("just made it");
    assert!(bound[0].names(object), "the object it was bound against");

    state
        .move_object(
            bear,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("it blinks out");
    state
        .move_object(
            bear,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("and back");
    assert!(
        !bound[0].names(state.object(bear).expect("same id")),
        "and not whatever is at that id later"
    );
}

/// A modifier that changes neither characteristics nor control has no
/// set to lock (CR 611.2c), so it stays the question it was written as:
/// one dynamic filter, which keeps catching whatever arrives.
#[test]
fn a_modifier_that_locks_nothing_stays_a_question() {
    let mut state = state();
    permanent(&mut state, me(), "Present");
    let source = permanent(&mut state, me(), "Source");

    let bound = bound_now(
        &state,
        &Filter::Any,
        &Modifier::DoesNotUntap,
        me(),
        source,
        None,
    );
    assert_eq!(bound.len(), 1);
    assert!(
        matches!(bound[0], EffectFilter::Dsl(f) if *f == Filter::Any),
        "the filter it was written with, unread"
    );
}

/// A filter that reaches past the battlefield stays dynamic whatever
/// the modifier does: enumerating it would mean walking every zone the
/// filter could mean, and narrowing to the battlefield alone would
/// silently drop the rest.
#[test]
fn a_filter_that_leaves_the_battlefield_is_not_enumerated() {
    static ELSEWHERE: Filter = Filter::And(&[Filter::CREATURE, Filter::InZone(ZoneRef::Graveyard)]);
    let mut state = state();
    let source = permanent(&mut state, me(), "Source");

    let bound = bound_now(
        &state,
        &ELSEWHERE,
        &Modifier::ModifyPT(1, 1),
        me(),
        source,
        None,
    );
    assert_eq!(bound.len(), 1);
    assert!(
        matches!(bound[0], EffectFilter::Dsl(f) if *f == ELSEWHERE),
        "a locking modifier, and still the question"
    );
}

/// `only` is the half no `Filter` can do: "creatures target player
/// controls" depends on a choice, and a filter is told the ability's
/// controller and its source and nothing else. The seat is known at
/// resolution, which is where CR 611.2c wants the set bound anyway.
#[test]
fn a_named_seat_narrows_the_set_no_filter_could() {
    let mut state = state();
    let mine = permanent(&mut state, me(), "Mine");
    let theirs = permanent(&mut state, them(), "Theirs");

    let both = bound_now(
        &state,
        &Filter::Any,
        &Modifier::ModifyPT(1, 1),
        me(),
        mine,
        None,
    );
    assert_eq!(named(&both), vec![mine, theirs]);

    let narrowed = bound_now(
        &state,
        &Filter::Any,
        &Modifier::ModifyPT(1, 1),
        me(),
        mine,
        Some(&[them()]),
    );
    assert_eq!(
        named(&narrowed),
        vec![theirs],
        "the seat the card named, and not the one that cast it"
    );
}
