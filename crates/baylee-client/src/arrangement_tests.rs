//! The switcher's keys and choices (DESIGN-v8 §2.3, §2.6), without a window.

use super::*;
use crate::keys::Fired;
use baylee_client_core::prefs::Action;

fn table(seats: usize) -> ArrangementFrame {
    ArrangementFrame {
        frame: Some(TableFrame::Wide),
        seats,
        flash: 0.0,
    }
}

fn press(
    actions: &[Action],
    digits: &[u32],
    duel: &mut Duel,
    settings: &mut crate::settings::ClientSettings,
    measured: &mut ArrangementFrame,
) -> bool {
    keys(Fired::of_actions(actions), digits, duel, settings, measured)
}

/// `P` opens the menu on the row in effect and holds the keyboard; `Esc`
/// and `P` again shut it; at two seats there is nothing to open.
#[test]
fn p_opens_the_menu_and_esc_shuts_it() {
    let mut duel = Duel::default();
    let mut settings = crate::settings::ClientSettings::default();
    let mut measured = table(4);
    assert!(press(
        &[Action::ArrangementMenu],
        &[],
        &mut duel,
        &mut settings,
        &mut measured
    ));
    assert_eq!(duel.arrangement_menu, Some(Arrangement::Ring.index()));
    assert!(
        press(
            &[Action::Confirm],
            &[],
            &mut duel,
            &mut settings,
            &mut measured
        ) || duel.arrangement_menu.is_none(),
        "while it stands, the menu has the keys"
    );
    duel.arrangement_menu = Some(0);
    assert!(press(
        &[Action::FocusNextSeat],
        &[],
        &mut duel,
        &mut settings,
        &mut measured
    ));
    assert_eq!(
        duel.visiting, None,
        "the table hears nothing under the menu"
    );
    press(
        &[Action::Cancel],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, None);
    press(
        &[Action::ArrangementMenu],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    press(
        &[Action::ArrangementMenu],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, None, "P again shuts it");

    let mut duel = Duel::default();
    let mut measured = table(2);
    press(
        &[Action::ArrangementMenu],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, None, "a duel has one arrangement");
    assert!(
        !press(
            &[Action::FocusHome],
            &[],
            &mut duel,
            &mut settings,
            &mut measured
        ),
        "with the menu shut, other keys are the table's"
    );
}

/// `↑↓` walk the rows and come round, the remember row included.
#[test]
fn the_arrows_walk_the_rows_round() {
    let mut duel = Duel::default();
    let mut settings = crate::settings::ClientSettings::default();
    let mut measured = table(4);
    duel.arrangement_menu = Some(0);
    press(
        &[Action::NumberUp],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, Some(REMEMBER_ROW));
    press(
        &[Action::NumberDown],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, Some(0));
    press(
        &[Action::CursorDown],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert_eq!(duel.arrangement_menu, Some(1));
}

/// `Shift+P` cycles the offered arrangements only: where the ring is the
/// one offered it changes nothing, and the pill flashes all the same.
#[test]
fn shift_p_cycles_only_what_is_offered_and_flashes() {
    let mut duel = Duel::default();
    let mut settings = crate::settings::ClientSettings::default();
    let mut measured = table(4);
    let offered = Arrangement::offered_at(4, TableFrame::Wide);
    press(
        &[Action::NextArrangement],
        &[],
        &mut duel,
        &mut settings,
        &mut measured,
    );
    assert!((measured.flash - FLASH_SECS).abs() < f32::EPSILON);
    let now = chosen(&duel, &settings.table, 4);
    assert!(offered.contains(&now), "{now:?} is offered");
    assert_eq!(now, Arrangement::Ring.next_offered(4, TableFrame::Wide));
    for _ in 0..offered.len() - 1 {
        press(
            &[Action::NextArrangement],
            &[],
            &mut duel,
            &mut settings,
            &mut measured,
        );
    }
    assert_eq!(
        chosen(&duel, &settings.table, 4),
        Arrangement::Ring,
        "past the last is the first"
    );
}

/// A digit chooses its row; a greyed row chooses nothing and leaves the
/// menu standing; an offered one shuts it.
#[test]
fn a_digit_chooses_its_row_and_a_greyed_row_nothing() {
    let mut duel = Duel::default();
    let mut settings = crate::settings::ClientSettings::default();
    let mut measured = table(4);
    let greyed = Arrangement::ALL
        .into_iter()
        .find(|a| a.offered(4, TableFrame::Wide).is_err());
    if let Some(greyed) = greyed {
        duel.arrangement_menu = Some(0);
        let digit = u32::try_from(greyed.index() + 1).expect("one digit");
        press(&[], &[digit], &mut duel, &mut settings, &mut measured);
        assert_eq!(
            duel.arrangement_game, None,
            "{greyed:?} is not offered here"
        );
        assert!(duel.arrangement_menu.is_some(), "the menu stays");
    }
    duel.arrangement_menu = Some(3);
    press(&[], &[1], &mut duel, &mut settings, &mut measured);
    assert_eq!(duel.arrangement_game, Some(Arrangement::Ring));
    assert_eq!(duel.arrangement_menu, None, "chosen: the menu shuts");
}

/// Unticked, a choice is this game's alone; ticked, it is the device's
/// memory for the seat count, and the per-game switch is dropped.
#[test]
fn remember_writes_the_seat_count_s_memory() {
    let mut duel = Duel::default();
    let mut settings = crate::settings::ClientSettings::default();
    let measured = table(5);
    assert!(pick(&mut duel, &mut settings, &measured, Arrangement::Ring));
    assert_eq!(duel.arrangement_game, Some(Arrangement::Ring));
    assert_eq!(settings.table.arrangement_by_seats.get(5), None);
    press_row(&mut duel, &mut settings, &measured, REMEMBER_ROW);
    assert!(duel.arrangement_remember);
    assert_eq!(
        settings.table.arrangement_by_seats.get(5),
        Some(Arrangement::Ring)
    );
    assert_eq!(duel.arrangement_game, None);
    press_row(&mut duel, &mut settings, &measured, REMEMBER_ROW);
    assert_eq!(
        settings.table.arrangement_by_seats.get(5),
        None,
        "unticked: forgotten"
    );
}

/// What is in effect: the game's switch over the per-count memory over the
/// default, and the ring wherever the choice is not offered.
#[test]
fn the_arrangement_in_effect_reads_game_then_count_then_default() {
    let mut duel = Duel::default();
    let mut view = TableView {
        arrangement: Arrangement::Pods,
        ..TableView::default()
    };
    view.arrangement_by_seats
        .set(4, Some(Arrangement::Spotlight));
    assert_eq!(chosen(&duel, &view, 4), Arrangement::Spotlight);
    assert_eq!(chosen(&duel, &view, 5), Arrangement::Pods);
    duel.arrangement_game = Some(Arrangement::UprightRing);
    assert_eq!(chosen(&duel, &view, 4), Arrangement::UprightRing);
    for frame in TableFrame::ALL {
        for seats in 2..=8 {
            let (effective, _) = in_effect(&duel, &view, seats, frame);
            assert!(
                effective.offered(seats, frame).is_ok(),
                "{effective:?} at {seats} on {frame:?}"
            );
        }
    }
    let (at_two, _) = in_effect(&duel, &view, 2, TableFrame::Wide);
    assert_eq!(at_two, Arrangement::Ring, "a duel is a duel");
}

/// The settings' per-count stepper walks *Default* and the built
/// arrangements, round both ways.
#[test]
fn the_per_count_stepper_walks_default_and_the_built_ones() {
    let built: Vec<Option<Arrangement>> = std::iter::once(None)
        .chain(Arrangement::ALL.into_iter().filter(|a| a.built()).map(Some))
        .collect();
    let mut at = None;
    for want in built.iter().cycle().skip(1).take(built.len()) {
        at = step_remembered(at, 1);
        assert_eq!(at, *want);
    }
    assert_eq!(at, None, "round to the default");
    assert_eq!(
        step_remembered(None, -1),
        *built.last().expect("one at least")
    );
}
