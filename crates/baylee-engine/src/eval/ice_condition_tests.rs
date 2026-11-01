use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use crate::event::GameEvent;
use crate::object::ObjectKind;
use crate::turn::{Phase, Step};
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{Amount, Condition};
use baylee_core::color::{Color, ColorSet};
use baylee_core::mana::ManaColor;

fn fixture() -> (GameState, ObjectId, PlayerId, PlayerId) {
    let mut state =
        GameState::from_preset(&preset(17, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let you = PlayerId::new(0);
    let them = PlayerId::new(1);
    let name = state.names.intern("Condition observer");
    let source = state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    (state, source, you, them)
}

#[test]
fn amount_parity_includes_zero_and_rejects_odd_values() {
    let (state, source, you, _) = fixture();
    for (amount, expected) in [(0, true), (1, false), (2, true), (u32::MAX, false)] {
        assert_eq!(
            condition_holds(
                &state,
                you,
                source,
                Condition::AmountIsEven(Amount::Fixed(amount))
            ),
            expected
        );
    }
}

#[test]
fn chosen_color_requires_a_unique_positive_plurality_on_the_chosen_players_board() {
    let (mut state, source, you, them) = fixture();
    state
        .object_mut(source)
        .unwrap()
        .set_chosen_opponent(Some(them));
    state.object_mut(source).unwrap().chosen_color = Some(ManaColor::Red);
    let condition = Condition::ChosenColorIsUniqueMostCommon(&Filter::Not(&Filter::IsToken));
    assert!(!condition_holds(&state, you, source, condition));
    let name = state.names.intern("Multicolor permanent");
    let red = state.create_bare(them, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    state
        .object_mut(red)
        .unwrap()
        .riders
        .push(crate::object::Rider::MergedToken(false));
    state.object_mut(red).unwrap().base_mut().colors = ColorSet::of(Color::Red);
    assert!(condition_holds(&state, you, source, condition));
    state.object_mut(red).unwrap().base_mut().colors =
        ColorSet::of(Color::Red).union(ColorSet::of(Color::Blue));
    assert!(
        !condition_holds(&state, you, source, condition),
        "a multicolor tie is not a majority"
    );
    let second = state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    state
        .object_mut(second)
        .unwrap()
        .riders
        .push(crate::object::Rider::MergedToken(false));
    state.object_mut(second).unwrap().base_mut().colors = ColorSet::of(Color::Red);
    assert!(
        !condition_holds(&state, you, source, condition),
        "another controller's permanent is irrelevant"
    );
    state.object_mut(second).unwrap().controller = them;
    assert!(condition_holds(&state, you, source, condition));
}

#[test]
fn blocked_history_advances_at_upkeep_without_a_resolving_trigger() {
    let (mut state, source, you, them) = fixture();
    let upkeep = |state: &mut GameState, active| {
        state.begin_upkeep_history(active);
        state.turn.active = active;
        state.turn.step = Step::Upkeep;
        state.journal.record(GameEvent::StepChanged {
            active,
            phase: Phase::Beginning,
            step: Step::Upkeep,
        });
    };
    upkeep(&mut state, you);
    let identity = state.source_identity(source).unwrap();
    state.record_blocked_or_blocking(identity, identity);
    upkeep(&mut state, them);
    upkeep(&mut state, you);
    assert!(super::upkeep_history::blocked_since_last_upkeep(
        &state, you, source
    ));
    // No ability resolution occurs before the next upkeep.
    upkeep(&mut state, them);
    upkeep(&mut state, you);
    assert!(!super::upkeep_history::blocked_since_last_upkeep(
        &state, you, source
    ));
    state.record_blocked_or_blocking(identity, identity);
    assert!(super::upkeep_history::blocked_since_last_upkeep(
        &state, you, source
    ));
}

#[test]
fn upkeep_history_uses_each_controllers_window_and_the_exact_incarnation() {
    let (mut state, source, you, them) = fixture();
    state.turn.step = Step::Main;
    let reference = state.source_identity(source).unwrap();
    state.record_blocked_or_blocking(reference, reference);
    state.begin_upkeep_history(you);
    assert!(!super::upkeep_history::blocked_since_last_upkeep(
        &state, you, source
    ));
    assert!(super::upkeep_history::blocked_since_last_upkeep(
        &state, them, source
    ));
    state.object_mut(source).unwrap().version += 1;
    assert!(!super::upkeep_history::blocked_since_last_upkeep(
        &state, them, source
    ));
}

#[test]
fn blocked_history_is_part_of_the_deterministic_state_hash() {
    let (mut state, source, _, _) = fixture();
    let before = state.snapshot_hash();
    let reference = state.source_identity(source).unwrap();
    state.record_blocked_or_blocking(reference, reference);
    assert_ne!(state.snapshot_hash(), before);
}

#[test]
fn life_total_overflow_reports_a_numeric_failure_without_wrapping() {
    let (mut state, _, you, _) = fixture();
    state.players[usize::from(you.get())].life = i32::MAX;
    state.change_life(you, 1, crate::event::Cause::Effect);
    assert_eq!(state.players[usize::from(you.get())].life, i32::MAX);
    assert!(state.numeric_failure.is_some());
}
