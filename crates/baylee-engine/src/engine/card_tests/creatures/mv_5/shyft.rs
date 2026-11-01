//! Shyft's optional upkeep color choice, with indefinite incarnation-bound results.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shyft() -> CardIndex {
    card_index("40c3007a-be6c-4400-9139-12b9ca098ac4")
}

fn upkeep_choice(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    assert_eq!(engine.state().turn.active, P0);
    assert_eq!(engine.state().turn.step, Step::Upkeep);
    assert!(matches!(engine.pending(), Pending::YesNo { player, .. } if *player == P0));
}

fn colors(engine: &Engine<RegistryLookup>, source: ObjectId) -> ColorSet {
    engine
        .state()
        .object(source)
        .unwrap()
        .characteristics()
        .colors
}

fn choose_colors(engine: &mut Engine<RegistryLookup>, wanted: ColorSet) {
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending()
    else {
        panic!("accepting Shyft's color change must ask for colors")
    };
    assert_eq!(*player, P0);
    assert_eq!(
        options.len(),
        31,
        "every nonempty combination of the five colors"
    );
    assert!(
        !options
            .iter()
            .any(|mode| mode.kind == CastModeKind::Colors(ColorSet::EMPTY))
    );
    let choice = options
        .iter()
        .position(|mode| mode.kind == CastModeKind::Colors(wanted))
        .unwrap();
    let before = engine.snapshot_hash();
    assert!(engine.apply(P0, PlayerAction::ChooseMode(31)).is_err());
    assert_eq!(engine.snapshot_hash(), before);
    engine.apply(P0, PlayerAction::ChooseMode(choice)).unwrap();
    pass_until(engine, stack_is_empty);
}

#[test]
fn shyft_casts_as_blue_then_replaces_its_colors_at_its_controllers_upkeep() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[shyft()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, shyft());
    pass_until(&mut engine, stack_is_empty);
    let source = on_battlefield(&engine, P0, shyft()).unwrap();
    assert_eq!(pt(&engine, source), (4, 2));
    assert_eq!(colors(&engine, source), ColorSet::of(Color::Blue));
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(colors(&engine, source), ColorSet::of(Color::Blue));
    upkeep_choice(&mut engine);
    let red_green = ColorSet::of(Color::Red).union(ColorSet::of(Color::Green));
    choose_colors(&mut engine, red_green);
    assert_eq!(colors(&engine, source), red_green);
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        colors(&engine, source),
        red_green,
        "the change survives cleanup"
    );
    upkeep_choice(&mut engine);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        colors(&engine, source),
        red_green,
        "declining preserves the earlier choice"
    );
    reach_their_main_phase(&mut engine, P1);
    upkeep_choice(&mut engine);
    choose_colors(&mut engine, ColorSet::of(Color::White));
    assert_eq!(
        colors(&engine, source),
        ColorSet::of(Color::White),
        "a later choice replaces all previous colors"
    );
}

#[test]
fn shyft_can_choose_all_five_colors_without_targeting_itself() {
    let mut engine = Duel::new(SEED, island()).battlefield(0, &[shyft()]).start();
    keep_mulligans(&mut engine);
    let source = on_battlefield(&engine, P0, shyft()).unwrap();
    upkeep_choice(&mut engine);
    let all = ColorSet::from_slice(&[
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ]);
    choose_colors(&mut engine, all);
    assert_eq!(colors(&engine, source), all);
}

#[test]
fn shyfts_old_upkeep_trigger_does_not_change_a_returned_shyft() {
    let mut engine = Duel::new(SEED, island()).battlefield(0, &[shyft()]).start();
    keep_mulligans(&mut engine);
    let source = on_battlefield(&engine, P0, shyft()).unwrap();
    upkeep_choice(&mut engine);
    let state = engine.dev_state_mut(P0).unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Exile(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(colors(&engine, source), ColorSet::of(Color::Blue));
}
