//! The shell's side of the guided tours (TOURS.md §1.4): which place the
//! player stands at, when a chapter opens there, the try-it checks read off
//! the lobby's state, and what the bubble's box and the practice offer ask.
//! The drawing and the tour's keys are `crate::tour`'s.

use baylee_client_core::tour::{Check, Kind, Place, Run, Tour};

use crate::tour::{Setting, TourAnchor, TourDesk, TourPress};

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// Where the player stands in the shell, as a tour's chapter knows places;
/// `None` on the settings screen and a screen no chapter opens on.
pub(super) fn place(state: &LobbyState) -> Option<Place> {
    if state.settings_open() {
        return None;
    }
    match state.lobby.screen() {
        Screen::SignIn { .. } => Some(Place::Door),
        Screen::Table if state.lobby.awaiting().is_some() && !state.room_away => Some(Place::Room),
        Screen::Table => Some(match state.hub {
            Hub::Play => Place::Play,
            Hub::Decks => Place::Shelf,
        }),
        Screen::Build => Some(Place::Builder),
        Screen::Seated(_) => None,
    }
}

/// Whether a try-it check holds now.
fn holds(check: Check, state: &LobbyState) -> bool {
    match check {
        Check::CreateSheetOpen => state.play.sheet.is_some(),
        Check::InRoom => place(state) == Some(Place::Room),
        // The builder's and the table's are answered where those are.
        _ => false,
    }
}

/// Whether something over the screen holds it now (a sheet of the shell's
/// own, the report form, the `?` overlay, the door's passage): no chapter
/// opens under it, and one standing is set aside.
fn covered(
    state: &LobbyState,
    entrance: &super::entrance::Entrance,
    journey: Option<&crate::arrival::Journey>,
    report: Option<&crate::report::ReportDesk>,
    overlay: Option<&crate::shellkit::overlay::Overlay>,
) -> bool {
    state.terms.up()
        || state.about_open
        || state.confirmation.is_some()
        || state.header_menu.is_some()
        || entrance.active()
        || journey.is_some_and(crate::arrival::Journey::active)
        || report.is_some_and(crate::report::ReportDesk::holds_keyboard)
        || overlay.is_some_and(crate::shellkit::overlay::holds)
}

/// Opens, sets aside and resumes the shell's chapters, answers their
/// checks, and carries out the box and the practice offer.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a Bevy system: the screen, what may cover it, in order
pub(super) fn tours(
    mut state: ResMut<LobbyState>,
    mut desk: ResMut<TourDesk>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
    windows: Query<&Window>,
    input: Option<Res<crate::shellkit::InputClass>>,
    entrance: Res<super::entrance::Entrance>,
    journey: Option<Res<crate::arrival::Journey>>,
    report: Option<Res<crate::report::ReportDesk>>,
    overlay: Option<Res<crate::shellkit::overlay::Overlay>>,
    anchors: Query<(&TourAnchor, &InheritedVisibility, &ComputedNode)>,
    (mut commands, mut opens): (Commands, MessageWriter<crate::DuelCommand>),
) {
    let Some(settings) = settings.as_mut() else {
        return;
    };
    // What the bubble asked of the lobby.
    match desk.asked.take() {
        Some(TourPress::NeverAgain) => {
            state.tours_before = desk.before.take();
            state.undo = Some(super::decks::Undo {
                kind: super::decks::UndoKind::ToursOff,
                left: super::decks::UNDO_SECS,
            });
        }
        // The practice game (TOURS.md §1.4): four seats, three house AIs,
        // hosted here, untimed; the table tour starts at its first view.
        Some(TourPress::Practice) => {
            let lang = baylee_client_core::Lang::of(&settings.lang);
            if let Some(host) =
                crate::host::practice_game(crate::host::fresh_seed()).and_then(|preset| {
                    let names = super::offline::seat_names(&preset, lang);
                    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                    crate::host::LocalHost::new(&preset, baylee_core::ids::PlayerId::new(0), &refs)
                })
            {
                commands.insert_resource(crate::InstalledHost(Box::new(host)));
                opens.write(crate::DuelCommand::Open);
                desk.practice = true;
            }
        }
        _ => {}
    }
    let (width, height) = windows
        .iter()
        .next()
        .map_or((1280.0, 800.0), |w| (w.width(), w.height()));
    let m = super::header::kit_metrics(
        width,
        height,
        settings.text_size,
        input.as_deref().copied().unwrap_or_default(),
    );
    let phone = m.frame == crate::shellkit::Frame::Phone;
    // Desktop only (owner, 09.10.): on a phone or under touch nothing
    // starts, and what stood is put away.
    if !baylee_client_core::tour::offered_here(phone, m.touch()) {
        if desk.run.as_ref().is_some_and(|r| r.tour != Tour::Table) {
            desk.run = None;
        }
        return;
    }
    let setting = Setting {
        top: m.header,
        phone,
        ..Setting::default()
    };
    if desk.setting != setting {
        desk.setting = setting;
    }
    let here = place(&state);
    let covered = covered(
        &state,
        &entrance,
        journey.as_deref(),
        report.as_deref(),
        overlay.as_deref(),
    );
    let there = |anchor: Option<baylee_client_core::tour::Anchor>| {
        anchor.is_none_or(|want| crate::tour::present(want, &anchors))
    };
    // A run standing: its check, and whether its screen is still up.
    if let Some(run) = desk.run.as_mut()
        && run.tour != Tour::Table
    {
        // Done what the step asked: the tour goes on by itself.
        if let Kind::Try(check) = run.current().kind
            && !run.held
            && holds(check, &state)
        {
            let mut tours = settings.tours.clone();
            if run.satisfied(&mut tours) == baylee_client_core::tour::Moved::Over {
                desk.run = None;
            }
            settings.tours = tours;
            settings.save();
            return;
        }
        let home = run.single || Some(run.current_chapter().place) == here;
        if !home && matches!(run.current().kind, Kind::Try(_)) && run.held {
            // The try-it step was the way off this screen (Open table into
            // the room): it is done.
            let mut tours = settings.tours.clone();
            if run.next(&mut tours) == baylee_client_core::tour::Moved::Over {
                desk.run = None;
            }
            settings.tours = tours;
            settings.save();
            return;
        }
        let parked = !home || covered;
        // Away from its screen, a chapter waits to be resumed there — unless
        // the screen the player is on has a chapter of its own, which opens
        // instead; the one set aside starts over on its next visit.
        let elsewhere = !home
            && !covered
            && here.is_some_and(|p| {
                [Tour::Lobby, Tour::Builder]
                    .into_iter()
                    .any(|t| settings.tours.due(t, p).is_some())
            });
        if !elsewhere {
            if desk.parked != parked {
                desk.parked = parked;
            }
            return;
        }
        desk.run = None;
        desk.parked = false;
    }
    if desk.run.is_some() || covered {
        return;
    }
    let Some(here) = here else {
        return;
    };
    let tours = &settings.tours;
    // A chapter of this place, once its first step's anchor stands.
    for tour in [Tour::Lobby, Tour::Builder] {
        if let Some(chapter) = tours.due(tour, here)
            && let Some(run) = Run::chapter(tour, chapter, phone, 0)
            && there(run.current().anchor_on(phone))
        {
            desk.start(run);
            return;
        }
    }
    // A just-in-time step whose anchor stands for the first time.
    if !tours.tips {
        return;
    }
    for tour in [Tour::Lobby, Tour::Builder] {
        for (c, chapter) in tour.chapters().iter().enumerate() {
            for (s, step) in chapter.steps.iter().enumerate() {
                if step.kind == Kind::Jit
                    && step.only.runs(phone, 0)
                    && !tours.step_seen(tour, step)
                    && step.anchor_on(phone).is_some()
                    && there(step.anchor_on(phone))
                {
                    desk.start(Run::jit(tour, c, s, phone));
                    return;
                }
            }
        }
    }
}
