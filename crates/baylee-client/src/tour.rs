//! The guided tours' renderer (`.claude/tours/TOURS.md` v3): the spotlight
//! (a scrim of four nodes round a hole), the bubble, the folded pill, and
//! the tour's own keys and presses.
//!
//! What a tour says and in what order is `baylee_client_core::tour`; this
//! draws a [`Run`] and asks for its anchors every frame. A screen's glue
//! (`lobby::touring` for the shell, `hud`'s for the table) writes the
//! [`Setting`] each frame — where the player is, whether a question waits —
//! starts the runs and answers the try-it checks.
//!
//! Anchors are a marker, [`TourAnchor`], that the **spawner** of a node puts
//! beside its other components; trees here are rebuilt often, so the tour
//! queries by id every frame and caches nothing (§3.1). A scripted step
//! whose anchor is not there when it would open is passed over, its dot
//! hollow (§3.3).

mod draw;
mod table;

use baylee_client_core::tour::{Anchor, Mode, Moved, Run, Tours};
use bevy::input::keyboard::KeyCode;
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;

pub use draw::Spotlight;

/// Marks the node a tour step points at. Put by the node's spawner, never
/// by the tour.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct TourAnchor(pub Anchor);

/// What the screen under the tour tells it, written every frame by the
/// screen's glue.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Setting {
    /// The `GlobalZIndex` of the scrim; the bubble stands five above it.
    pub z: i32,
    /// The scrim's darkness (.55 over the shell, .45 over the table).
    pub alpha: f32,
    /// The top of the room the bubble may use (under a header or
    /// `TOP_CLEAR`), where the pill stands.
    pub top: f32,
    /// A question waits for this seat (the table).
    pub pending: bool,
    /// Phone-sized.
    pub phone: bool,
    /// The pill's line under its title: "the table waits".
    pub at_table: bool,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            z: crate::shellkit::tokens::z::SHEET - 1,
            alpha: 0.55,
            top: 64.0,
            pending: false,
            phone: false,
            at_table: false,
        }
    }
}

/// The tour standing, if one is.
#[derive(Resource, Default)]
pub struct TourDesk {
    /// The run, if a tour is running.
    pub run: Option<Run>,
    /// The run is set aside: its screen is not up (the player navigated
    /// away), so nothing is drawn and the keys are the screen's. It comes
    /// back where it was.
    pub parked: bool,
    /// What the screen says this frame.
    pub setting: Setting,
    /// The key that closed the bubble is not also the screen's (one frame).
    swallow: bool,
    /// How long the current step's anchor has been missing.
    missing: f32,
    /// And for how many frames: a window drawing one frame a second (in the
    /// background) must not pass a step over before its tree is rebuilt.
    missing_frames: u32,
    /// A press or a key's act for the glue to carry out (the box, the
    /// practice game), taken by it.
    pub asked: Option<TourPress>,
    /// The tours before the box switched them off, for its Undo.
    pub before: Option<Tours>,
    /// A practice game was just installed: the table tour starts at its
    /// first view (`table::tours`).
    pub practice: bool,
    /// The scripted run a just-in-time step stepped in front of; it comes
    /// back when that step is done.
    pub stash: Option<Run>,
}

impl TourDesk {
    /// Whether the keyboard is the tour's this frame: a narrated bubble
    /// stands, or it closed on this frame's key. Every key handler under it
    /// asks this, as it asks the report form's.
    #[must_use]
    pub fn holds_keyboard(&self) -> bool {
        self.swallow || (!self.parked && self.run.as_ref().is_some_and(Run::holds_keyboard))
    }

    /// The run, if one is drawn now.
    #[must_use]
    pub fn shown(&self) -> Option<&Run> {
        self.run.as_ref().filter(|_| !self.parked)
    }

    /// Starts a run, unless one stands.
    pub fn start(&mut self, run: Run) {
        if self.run.is_none() {
            self.run = Some(run);
            self.parked = false;
            self.missing = 0.0;
            self.missing_frames = 0;
        }
    }

    /// Carries out one of the bubble's own presses on the run; what is the
    /// glue's (the box, the practice game, Later) is left in [`Self::asked`].
    pub fn act(&mut self, press: TourPress, tours: &mut Tours) -> bool {
        let Some(run) = self.run.as_mut() else {
            return false;
        };
        // The offer's primary is the practice game, by key as by press.
        let press = if press == TourPress::Next
            && run.current().kind == baylee_client_core::tour::Kind::Offer
        {
            TourPress::Practice
        } else {
            press
        };
        let over = match press {
            TourPress::Next if run.primary_live() => run.next(tours) == Moved::Over,
            TourPress::Next => false,
            TourPress::Back => {
                run.back();
                false
            }
            TourPress::Skip => run.skip(tours) == Moved::Over,
            TourPress::Fold => {
                run.fold();
                false
            }
            TourPress::Unfold => {
                run.unfold();
                false
            }
            TourPress::NeverAgain => {
                self.before = Some(tours.clone());
                tours.set_all(false);
                self.asked = Some(press);
                true
            }
            TourPress::Practice | TourPress::Later | TourPress::LeaveTable => {
                run.next(tours);
                self.asked = Some(press);
                true
            }
        };
        self.missing = 0.0;
        self.missing_frames = 0;
        if over {
            self.end();
            self.swallow = true;
        }
        true
    }

    /// The run is over: the one a just-in-time step stood in front of, if
    /// any, comes back.
    pub fn end(&mut self) {
        self.run = self.stash.take();
        self.parked = false;
        self.missing = 0.0;
        self.missing_frames = 0;
    }

    /// A just-in-time step steps in front of whatever stands.
    pub fn interject(&mut self, run: Run) {
        if let Some(standing) = self.run.take()
            && !standing.single
        {
            self.stash = Some(standing);
        }
        self.run = Some(run);
        self.parked = false;
        self.missing = 0.0;
        self.missing_frames = 0;
    }
}

/// What a control of the bubble or the pill does.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TourPress {
    /// Next, or Done on the last step.
    Next,
    /// Back one step.
    Back,
    /// Skip the chapter.
    Skip,
    /// Fold to the pill.
    Fold,
    /// The pill: unfold.
    Unfold,
    /// The box: every tour and tip off.
    NeverAgain,
    /// L14: start the practice game.
    Practice,
    /// L14: later.
    Later,
    /// The room chapter's last step: leave the table the tour opened (the
    /// glue asks it as the room's own Leave does).
    LeaveTable,
}

/// The tour's keys while a narrated bubble stands (TOURS.md §1.3): `→`,
/// `Enter`, `Space` = Next; `←` = Back; `Esc` = fold, never skip.
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut desk: ResMut<TourDesk>,
    settings: Option<ResMut<crate::settings::ClientSettings>>,
    report: Option<Res<crate::report::ReportDesk>>,
    overlay: Option<Res<crate::shellkit::overlay::Overlay>>,
) {
    if desk.swallow {
        desk.swallow = false;
    }
    if !desk.shown().is_some_and(Run::holds_keyboard)
        || report.is_some_and(|r| r.holds_keyboard())
        || overlay.is_some_and(|o| crate::shellkit::overlay::holds(&o))
    {
        return;
    }
    let press = if keys.any_just_pressed([
        KeyCode::ArrowRight,
        KeyCode::Enter,
        KeyCode::NumpadEnter,
        KeyCode::Space,
    ]) {
        TourPress::Next
    } else if keys.just_pressed(KeyCode::ArrowLeft) {
        TourPress::Back
    } else if keys.just_pressed(KeyCode::Escape) {
        TourPress::Fold
    } else {
        return;
    };
    let Some(mut settings) = settings else {
        return;
    };
    let before = settings.tours.clone();
    let mut tours = before.clone();
    desk.act(press, &mut tours);
    desk.swallow = true;
    if tours != before {
        settings.tours = tours;
        settings.save();
    }
}

/// The bubble's and the pill's presses.
fn presses(
    mut clicks: MessageReader<Pointer<Click>>,
    buttons: Query<&TourPress>,
    disabled: Query<(), With<crate::shellkit::controls::Disabled>>,
    parents: Query<&ChildOf>,
    mut desk: ResMut<TourDesk>,
    settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    let mut pressed = None;
    for click in clicks.read() {
        let mut at = click.entity;
        for _ in 0..8 {
            if disabled.contains(at) {
                break;
            }
            if let Ok(press) = buttons.get(at) {
                pressed = Some(*press);
                break;
            }
            match parents.get(at) {
                Ok(parent) => at = parent.parent(),
                Err(_) => break,
            }
        }
    }
    let (Some(press), Some(mut settings)) = (pressed, settings) else {
        return;
    };
    let before = settings.tours.clone();
    let mut tours = before.clone();
    desk.act(press, &mut tours);
    if tours != before {
        settings.tours = tours;
        settings.save();
    }
}

/// Follows the screen's question (TOURS.md §1.5) and passes over a step
/// whose anchor is not there (§3.3).
fn follow(
    time: Res<Time>,
    mut desk: ResMut<TourDesk>,
    anchors: Query<(&TourAnchor, &InheritedVisibility, &ComputedNode)>,
    settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    let pending = desk.setting.pending;
    let phone = desk.setting.phone;
    if desk.parked {
        return;
    }
    let Some(run) = desk.run.as_mut() else {
        return;
    };
    if run.mode == Mode::Folded && !run.auto_folded {
        return;
    }
    run.follow_question(pending);
    let Some(anchor) = run.current().anchor_on(phone) else {
        desk.missing = 0.0;
        return;
    };
    if present(anchor, &anchors) {
        if desk.missing != 0.0 || desk.missing_frames != 0 {
            desk.missing = 0.0;
            desk.missing_frames = 0;
        }
        return;
    }
    desk.missing += time.delta_secs();
    desk.missing_frames += 1;
    // A rebuilt tree lands a frame or two after its screen: a grace, in
    // time and in frames, then the step is passed over, never pointed at
    // nothing.
    if desk.missing < MISSING_GRACE || desk.missing_frames < MISSING_FRAMES {
        return;
    }
    desk.missing = 0.0;
    desk.missing_frames = 0;
    let Some(mut settings) = settings else {
        return;
    };
    let mut tours = settings.tours.clone();
    let over = desk
        .run
        .as_mut()
        .is_some_and(|run| run.pass_over(&mut tours) == Moved::Over);
    if over {
        desk.end();
    }
    if tours != settings.tours {
        settings.tours = tours;
        settings.save();
    }
}

/// Whether a node marked `anchor` is on screen: shown, and laid out with a
/// size (a root kept empty while it has nothing to draw is not there).
#[must_use]
pub fn present(
    anchor: Anchor,
    anchors: &Query<(&TourAnchor, &InheritedVisibility, &ComputedNode)>,
) -> bool {
    anchors
        .iter()
        .any(|(a, shown, node)| a.0 == anchor && shown.get() && node.size() != Vec2::ZERO)
}

/// How long a step waits for its anchor before it is passed over.
const MISSING_GRACE: f32 = 0.6;
/// And how many frames, at the least.
const MISSING_FRAMES: u32 = 12;

/// Whether the tour's systems may run: a settings file to keep marks in.
fn settled(settings: Option<Res<crate::settings::ClientSettings>>) -> bool {
    settings.is_some()
}

/// Installs the desk, its keys and presses, and its drawing.
pub(crate) fn install(app: &mut App) {
    if app.world().contains_resource::<TourDesk>() {
        return;
    }
    app.init_resource::<TourDesk>()
        .init_resource::<draw::Spotlight>()
        .add_systems(
            Update,
            (
                (
                    table::mark,
                    table::mark_the_first_card,
                    table::proxies,
                    table::tours,
                )
                    .chain()
                    .run_if(in_state(crate::DuelPhase::Playing)),
                keys,
                presses,
                follow,
                draw::draw,
            )
                .chain()
                .run_if(settled),
        )
        .add_systems(OnEnter(crate::DuelPhase::Closed), table::leave)
        .add_systems(
            PostUpdate,
            draw::place
                .after(bevy::ui::UiSystems::Layout)
                .run_if(settled),
        );
}
