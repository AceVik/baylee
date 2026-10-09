//! The pointer: a click on a lobby control turned into an intent.

use super::press::{Cx, in_lineage, in_lineage_entity};
use super::systems::keep_gateways;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// Signs out: the session ends on the gateway as well as here, and a guest's
/// with the guest, which this device then keeps no longer (#269).
///
/// Offline has no account to forget, so the same button is what leaves
/// offline play — and the performer has to go with it, or the sign-in form's
/// own requests would still be answered out of the local deck file.
pub(super) fn sign_out(
    state: &mut LobbyState,
    prefs: &mut crate::prefs::Prefs,
    scrolled: &mut Scrolled,
    mailbox: &Mailbox,
    settings: &mut Option<ResMut<crate::settings::ClientSettings>>,
) {
    let guest = state.lobby.guest();
    // A deletion still waiting for its Undo goes out while the session is
    // still the account's (S-10).
    let waiting = state.lobby.flush_delete();
    dispatch(state, mailbox, waiting);
    state.undo = None;
    state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
    prefs.detach();
    scrolled.set(List::Table, 0.0);
    state.offline = None;
    let ending = state.lobby.sign_out();
    if guest {
        let gateway = state.gateway.clone();
        state.guests.remove(&gateway);
        if let Some(settings) = settings.as_mut() {
            keep_gateways(state, settings);
        }
    }
    dispatch(state, mailbox, ending);
}

/// A press that outlives the control it landed on.
///
/// The tree is rebuilt from the state, and a press can be what changes the
/// state: a press on a button while a field holds the caret moves the kit's
/// focus onto the button (`shellkit::focus::pointer_focus`), which parks the
/// lobby's caret (`front::keys::kit_to_lobby`), which rebuilds the face —
/// all between the button going down and coming up. Bevy's `Click` is a
/// release on the *entity* that was pressed, and that entity is gone by then,
/// so the first click after a field was focused did nothing and the second
/// one worked (the owner's "two clicks", 09.10.2026). A control is its
/// `Press`, not its entity: a release on the control drawn again in its
/// place, with the same `Press`, is the click the rebuild ate.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Pressing<'w, 's> {
    downs: MessageReader<'w, 's, Pointer<bevy::picking::events::Press>>,
    ups: MessageReader<'w, 's, Pointer<Release>>,
    /// The control the primary button went down on, and what it presses.
    held: Local<'s, Option<(Entity, Press)>>,
}

impl Pressing<'_, '_> {
    /// Forgets this frame's presses and releases (a panel in motion).
    fn clear(&mut self) {
        self.downs.clear();
        self.ups.clear();
        *self.held = None;
    }

    /// The controls whose click a rebuild ate this frame: released on a
    /// control drawn in place of the one pressed, which no longer exists,
    /// with the same `Press`.
    fn eaten(&mut self, presses: &Query<&Press>, parents: &Query<&ChildOf>) -> Vec<Entity> {
        for down in self.downs.read() {
            if down.button != PointerButton::Primary {
                continue;
            }
            *self.held = in_lineage_entity(down.entity, presses, parents)
                .and_then(|e| presses.get(e).ok().map(|p| (e, *p)));
        }
        let mut eaten = Vec::new();
        for up in self.ups.read() {
            if up.button != PointerButton::Primary {
                continue;
            }
            let Some((was, press)) = self.held.take() else {
                continue;
            };
            // Still there: bevy's own click answers it.
            if presses.contains(was) {
                continue;
            }
            if let Some(now) = in_lineage_entity(up.entity, presses, parents)
                && presses.get(now).is_ok_and(|p| *p == press)
            {
                eaten.push(now);
            }
        }
        eaten
    }
}

/// Turns a click on a lobby control into an intent.
///
/// What every press passes through is here; what one press does is in its
/// screen's handler (`FrontPress::handle`, `HubPress::handle`, …).
#[allow(clippy::too_many_arguments)] // two pointer streams, then the usual
pub(super) fn clicks(
    (mut pointer, mut pressing): (MessageReader<Pointer<Click>>, Pressing),
    mut ends: MessageReader<Pointer<DragEnd>>,
    mut scrolled: ResMut<Scrolled>,
    presses: Query<&Press>,
    disabled: Query<(), With<crate::shellkit::controls::Disabled>>,
    // The filter builder's own buttons carry the model's vocabulary rather
    // than a `Press`, and one query reads all of them — the same one the zone
    // browser reads. `crate::filterui` puts a `FilterAct` on every button it
    // draws, so a control added to the builder is wired by being drawn; the
    // alternative was a `Press` variant per button, in two places, kept in
    // step by hand.
    acts: Query<&crate::filterui::FilterAct>,
    dones: Query<&crate::filterui::FilterDone>,
    parents: Query<&ChildOf>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mailbox: Res<Mailbox>,
    // Absent in a headless test, which has no settings file to write to.
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
    motion: Res<super::front::FrontMotion>,
    entrance: Res<super::entrance::Entrance>,
    journey: Option<Res<crate::arrival::Journey>>,
    // Shift on a pool row's `+` sends the card to the other list (§7).
    codes: Option<Res<ButtonInput<KeyCode>>>,
) {
    // A panel on its way out or in answers nothing: what is under the
    // pointer is half of a form that is going, or not yet there.
    if journey.as_ref().is_some_and(|j| j.active()) || motion.moving() || entrance.active() {
        pointer.clear();
        ends.clear();
        pressing.clear();
        return;
    }
    let eaten = pressing.eaten(&presses, &parents);
    // A release always fires a click, drag or no drag, so a swipe down the
    // card list would add whichever card it started on. The scroll it already
    // performed is what the gesture meant.
    let swiped = ends.read().any(|end| end.distance.length() > DRAG_SLOP);
    if swiped {
        pointer.clear();
        return;
    }
    let clicked: Vec<Entity> = pointer.read().map(|click| click.entity).collect();
    for target in clicked.into_iter().chain(eaten) {
        // The builder's buttons first: its rows sit inside the deck builder's
        // own panel, so a `Press` above them would otherwise swallow a click
        // meant for a row.
        if let Some(act) = crate::input::find_in_lineage(target, &acts, &parents) {
            let act = act.0;
            state.lobby.builder_mut().filter_act(act);
            // *Done* is both: the act hands the row's caret back and the
            // marker beside it shuts the panel.
            if crate::input::find_in_lineage(target, &dones, &parents).is_some() {
                state.lobby.builder_mut().close_panel();
            }
            continue;
        }
        // A kit control that is drawn but off keeps its press so it can take
        // focus and say why; a click on it does nothing (§2.4).
        if in_lineage_entity(target, &presses, &parents).is_some_and(|e| disabled.contains(e)) {
            continue;
        }
        let Some(&press) = in_lineage(target, &presses, &parents) else {
            continue;
        };
        let shift = codes
            .as_deref()
            .is_some_and(|c| c.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]));
        let press = match press {
            Press::Build(BuildPress::AddFromPool(slot, false)) if shift => {
                Press::Build(BuildPress::AddFromPool(slot, true))
            }
            other => other,
        };
        let cx = Cx {
            state: &mut state,
            prefs: &mut prefs,
            scrolled: &mut scrolled,
            mailbox: &mailbox,
            settings: &mut settings,
        };
        run(press, cx);
    }
}

/// What a press does, whether a click or a key brought it (`Enter` on a
/// focused control, `front::keys::activate_by_key`): the guards every press
/// passes, then its screen's handler.
#[allow(clippy::too_many_lines)] // the guards every press passes, read top to bottom
pub(super) fn run(press: Press, cx: Cx<'_, '_, '_, '_, '_>) {
    let Cx {
        state,
        prefs,
        scrolled,
        mailbox,
        settings,
    } = cx;
    {
        let press = &press;
        // A sheet over the front door or the lobby holds the screen: only
        // its own controls answer (the terms are answered before anything
        // else; About is closed or followed to the source).
        // The account deletion's confirmation stands over the terms when
        // Decline asked for it, and its shade covers everything under it:
        // its own controls answer then.
        if state.terms.up()
            && state.lobby.deleting_account().is_none()
            && !matches!(
                press,
                Press::Front(
                    FrontPress::TermsAccept
                        | FrontPress::TermsNotNow
                        | FrontPress::TermsStay
                        | FrontPress::TermsRetry
                        | FrontPress::TermsDecline
                ) | Press::Shared(SharedPress::PickerNothing)
            )
        {
            return;
        }
        if state.about_open
            && !matches!(
                press,
                Press::Front(FrontPress::About(_) | FrontPress::OpenSource)
                    | Press::Shared(SharedPress::PickerNothing)
            )
        {
            return;
        }
        if state.confirmation.is_some()
            && !matches!(
                press,
                Press::Shared(SharedPress::ConfirmDestructive | SharedPress::CancelDestructive)
            )
        {
            return;
        }
        // Anything pressed but the menu's own controls closes the gear menu,
        // the veil around it included.
        if state.front_menu
            && !matches!(
                *press,
                Press::Front(FrontPress::FrontMenu)
                    | Press::Shared(SharedPress::PickLang(_) | SharedPress::PickerNothing)
            )
        {
            state.front_menu = false;
        }
        // A builder menu closes on any press that is not the builder's own
        // (those close it themselves, `BuildPress::handle`).
        if state.build.menu.is_some() && !matches!(press, Press::Build(_)) {
            state.build.menu = None;
        }
        // Anything but the header's own controls closes a header popover.
        if state.header_menu.is_some() && !matches!(*press, Press::Header(_)) {
            state.header_menu = None;
        }
        // Anything but a menu's own opener closes the menu open; an item
        // acts first and closes it the same way.
        if state.menu.is_some() && !matches!(press, Press::Shared(SharedPress::OpenMenu(_))) {
            state.menu = None;
        }
        // Any other control answers the question the back button asked.
        //
        // Every write in this preamble is guarded: taking `&mut` out of the
        // state marks it changed, and a changed state rebuilds the whole tree
        // (§10 #1 of the shell design) — so a press that changes nothing
        // must not write anything either (`a_press_that_changes_nothing_marks_nothing`).
        if *press != Press::Build(BuildPress::CloseBuilder) && state.confirm_leave {
            state.confirm_leave = false;
        }
        // A filter that changes what is in the list puts it back at the top:
        // finding yourself halfway down a fresh search is disorienting, and
        // the row you were reading is not in it any more anyway.
        if matches!(
            *press,
            Press::Build(
                BuildPress::ToggleColor(_)
                    | BuildPress::SetKind(_)
                    | BuildPress::SetCmc(_)
                    | BuildPress::TogglePlayable
                    | BuildPress::CycleSort
                    | BuildPress::ClearFilters
            )
        ) {
            scrolled.set(List::Pool, 0.0);
        }
        // Any click that is not the rebinding chip itself calls off a
        // rebinding in progress. Leaving it armed would mean the next key
        // pressed anywhere lands on whichever row was last tapped.
        if state.settings.capturing().is_some()
            && !matches!(*press, Press::Settings(SettingsPress::Rebind(_)))
        {
            state.settings = SettingsPane::Open;
        }
        // And anything but the seat panel's own controls takes the caret out
        // of its box.
        if !matches!(
            *press,
            Press::Settings(SettingsPress::Seat(_) | SettingsPress::SeatKey(_))
        ) && state.seat.typing()
        {
            state.seat.blur();
        }
        let cx = Cx {
            state,
            prefs,
            scrolled,
            mailbox,
            settings,
        };
        super::press::run(*press, cx);
    }
}

/// How far a pointer has to travel before the gesture is a scroll rather than
/// a tap. Below it a shaky finger would still add a card; above it, a swipe
/// down a list would.
const DRAG_SLOP: f32 = 8.0;
