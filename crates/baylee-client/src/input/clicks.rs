//! Clicks on menus, sheets and the browser, and presses outside them.

#[allow(clippy::wildcard_imports)] // the input module's shared vocabulary
use super::*;

/// A button on the shelf, or a row in the game menu it opens.
///
/// `pub(crate)` so that a test outside this module can answer the way a click
/// answers rather than by writing the flag a click would have written — the
/// same rule as a test that builds its own `PlayerAction`, one level down.
///
/// `was_armed` is the concession's state *before* this click, taken once at
/// the top of the loop: every click disarms, so the second press only counts
/// when nothing happened in between.
pub(crate) fn menu_click(duel: &mut Duel, action: MenuAction, was_armed: bool) {
    match action {
        MenuAction::ToggleGrantedActions
        | MenuAction::PickGranted(_)
        | MenuAction::ConfirmGranted => crate::hud::granted_click(duel, action),
        MenuAction::SortHand(order) => {
            duel.hand_order = order;
            duel.hand_scroll = 0.0;
            duel.hovered = None;
            duel.hovered_at = None;
            if let (Some(board), Some(view)) = (&mut duel.board, &duel.view) {
                duel.hand_groups = order.apply(&mut board.hand, view);
            }
        }
        MenuAction::ScrollHand(direction) => {
            duel.hand_scroll = (duel.hand_scroll + f32::from(direction) * 480.0).max(0.0);
            duel.hovered = None;
            duel.hovered_at = None;
        }
        // The one gesture here that is about the interface rather than the
        // game. The panel it opens keeps its own state in `Duel` for the
        // reason `ability_menu` does: what is open is the client's business,
        // and the renderer reads it rather than owning it.
        MenuAction::ToggleGameMenu => duel.game_menu = !duel.game_menu,
        MenuAction::ToggleLog => duel.log_open = !duel.log_open,
        MenuAction::ToggleAiLog => duel.ai_log_open = !duel.ai_log_open,
        MenuAction::Report => duel.report_asked = true,
        // The game menu shuts and the arrangement menu opens in its place;
        // `arrangement::choose` opens it, because it holds the settings.
        MenuAction::ArrangementMenu => {
            duel.game_menu = false;
            duel.arrangement_menu_asked = true;
        }
        // Two presses, because there is no undo behind this one. The panel
        // stays open between them — nothing here closes it — which is the
        // whole reason it is not a child of the shelf: the arming press
        // rebuilds the shelf's columns.
        MenuAction::Concede => {
            if was_armed {
                duel.submit(PlayerAction::Concede);
                // And shuts on the way out. The game is over, so there is
                // nothing left in the panel to press; `sync_menu` reads the
                // ending and would close it a frame later anyway, and saying
                // it here is what keeps the two from disagreeing about the
                // frame in between.
                duel.game_menu = false;
            } else {
                duel.concede_armed = true;
            }
        }
        // Re-checked and not merely drawn greyed: a button drawn a frame ago
        // must not send what the engine has since withdrawn, which is the same
        // rule the ability chooser follows. Draw offers still need mutual
        // agreement — a protocol item.
        MenuAction::OfferDraw => {
            if duel.can_offer_draw() {
                duel.submit(PlayerAction::OfferDraw);
                // An offer is made once and answered elsewhere, so the panel
                // has nothing left to say. The refusal above is deliberately
                // *not* a close: a press the engine turned down leaves the
                // player where they were.
                duel.game_menu = false;
            }
        }
        // Through the same door as the keys, and re-checked for the same
        // reason: the button is only drawn while a hold is running, and
        // `hold_action` reads the *current* view rather than the one that was
        // drawn — so a hold the engine has already expired cannot be
        // "cancelled" into a new one by a stale button.
        //
        // It takes the **autopilot** with it, which the pills in the corner
        // never did: AX §4.4 draws one picture for both, so one button has to
        // answer for both or the sentence would stay on the shelf with the
        // press having done nothing visible. The autopilot is entirely the
        // client's and reaches no wire, so it is simply dropped; nothing else
        // ends it but its own arrival at the next turn.
        MenuAction::ReleaseHold => {
            duel.autopilot = None;
            if duel.priority_held()
                && let Some(action) = duel.hold_action(false)
            {
                duel.submit(action);
            }
        }
        // The mirror of the line above, down to the road it takes:
        // `hold_action(false)` is what F6 sends, and which of the two things
        // it sends is decided by the *current* view. So the button's own
        // condition is re-read here — a stack that emptied, or a hold that
        // started, since the shelf was drawn turns this press into a promise
        // to do nothing or into a cancellation, and neither is what the cap
        // says.
        // Declining a payment: the window's own pass, sent without a tap.
        MenuAction::DeclinePayment => {
            if duel.paying() && duel.mana_run.is_none() {
                duel.submit(PlayerAction::PassPriority);
            }
        }
        MenuAction::HoldForStack => {
            if duel.can_hold_for_stack()
                && let Some(action) = duel.hold_action(false)
            {
                duel.submit(action);
            }
        }
        // The same door the keys use, so the two ways of confirming cannot
        // drift; `fire_armed` re-resolves against the current `LegalActions`.
        MenuAction::SendArmed => fire_armed(duel),
        MenuAction::CancelArmed => disarm(duel),
    }
}

/// A press anywhere that is neither the sheet nor its card closes the sheet.
///
/// The one gesture on this surface that is about *nothing*: every branch of
/// [`pointer`] answers a thing that was clicked, and this answers a click that
/// found nothing to answer it. So it cannot be a branch there —
/// `Pointer<Click>` is only ever raised on an entity that was hit, and the
/// felt is `Pickable::IGNORE`, so a click on bare cloth raises no message at
/// all. What it reads instead is the press and [`HoverMap`], which is the one
/// place that knows the pointer is over **nothing**.
///
/// The press and not the click, because the two land on different frames and
/// the release is what `pointer` reads: closing on the press and letting the
/// release fall through is what makes a click on *another* card close this
/// sheet and open that one, rather than doing both to the same card.
///
/// Two exemptions, which are the owner's own words for it — the dialog and
/// the card. Neither of them is *answering* anything: the card is where the
/// sheet came from and a second tap on it is a no-op ([`activate_card`] makes
/// sure of that), so a player rummaging around the permanent they are reading
/// about cannot lose their place.
pub fn close_the_sheet_on_a_press_outside_it(
    buttons: Res<ButtonInput<MouseButton>>,
    hovers: Res<bevy::picking::hover::HoverMap>,
    sheet: Query<&crate::hud::AbilitySheetRoot>,
    cards: Query<&CardVisual>,
    hand_cards: Query<&HandCardVisual>,
    parents: Query<&ChildOf>,
    mut duel: ResMut<Duel>,
) {
    // Whichever model has the paper. The card exemption has to follow it: a
    // cast chooser stands beside a card in the **hand**, and a rule that
    // looked only at the table's cards would close it the moment the player
    // reached back to the card it is about.
    let object = duel
        .cast_menu
        .as_ref()
        .map(|menu| menu.card)
        .or(duel.ability_menu);
    let Some(object) = object else {
        return;
    };
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    for hovered in hovers.values().flat_map(|over| over.keys().copied()) {
        let spared = find_in_lineage(hovered, &sheet, &parents).is_some()
            || find_in_lineage(hovered, &cards, &parents).is_some_and(|v| v.object == object)
            || find_in_lineage(hovered, &hand_cards, &parents).is_some_and(|h| h.object == object);
        if spared {
            return;
        }
    }
    duel.ability_menu = None;
    duel.cast_menu = None;
}

/// A press anywhere that is neither the game menu nor its own button shuts it.
///
/// [`close_the_sheet_on_a_press_outside_it`]'s sibling, on the same two
/// mechanics and for the same reason: `Pointer<Click>` is only ever raised on
/// an entity that was hit, so a press on bare felt raises nothing at all, and
/// [`HoverMap`] is the one place that knows the pointer is over **nothing**.
/// It is the press and not the click, for that function's reason as well — a
/// press that opens something else has to find this one already shut.
///
/// Two exemptions. The panel itself, obviously — a press on a row is answered
/// by [`menu_click`], and a press on its padding is answered by nobody, which
/// is exactly what a panel's padding is for. And the **burger**: the press
/// and the click it becomes land on different frames, so closing on the press
/// would hand the click a shut menu to re-open, and the button would stop
/// closing what it opened.
///
/// A press outside also disarms a half-pressed concession, because every
/// press does. That is not this function's doing and is worth not undoing: a
/// player who has gone somewhere else has left the decision, and the panel
/// coming back up at "Aufgeben" rather than at "Aufgeben? Nochmal drücken" is
/// the safe way round.
///
/// [`HoverMap`]: bevy::picking::hover::HoverMap
pub fn close_the_menu_on_a_press_outside_it(
    buttons: Res<ButtonInput<MouseButton>>,
    hovers: Res<bevy::picking::hover::HoverMap>,
    panel: Query<&crate::hud::MenuPanel>,
    buttons_on_screen: Query<&crate::hud::MenuButton>,
    parents: Query<&ChildOf>,
    mut duel: ResMut<Duel>,
) {
    if !duel.game_menu || !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    for hovered in hovers.values().flat_map(|over| over.keys().copied()) {
        let spared = find_in_lineage(hovered, &panel, &parents).is_some()
            || find_in_lineage(hovered, &buttons_on_screen, &parents)
                .is_some_and(|b| b.action == MenuAction::ToggleGameMenu);
        if spared {
            return;
        }
    }
    duel.game_menu = false;
}

/// A click on the ability sheet.
///
/// Its own function for the reason [`browser_click`] is: they are one widget,
/// and [`pointer`] is a routing table rather than a place where behaviour
/// lives.
///
/// Returns whether the click belonged to the sheet.
pub(super) fn sheet_click(
    duel: &mut Duel,
    entity: Entity,
    sheet: &SheetWidgets,
    parents: &Query<&ChildOf>,
) -> bool {
    // The cross comes first. It sits inside the sheet's head, so a branch
    // that matched a row before it would still be right — but it is the one
    // thing here that is not about an ability, and reading it first says so.
    if find_in_lineage(entity, &sheet.close, parents).is_some() {
        // Exactly what `Action::Cancel` does in [`ability_menu_keys`], and no
        // more: an armed deed survives the sheet closing, there as here,
        // because the card itself still carries it and taking it back is
        // `Esc` on the *table*.
        //
        // Both models, because this sheet draws both since the cast chooser
        // moved onto it: the cross is one control on one piece of paper, and
        // a cross that only closed one of them would be a dead button on the
        // other. `cast_menu_keys` answers `Action::Cancel` the same way.
        duel.ability_menu = None;
        duel.cast_menu = None;
        return true;
    }
    if let Some(button) = find_in_lineage(entity, &sheet.rows, parents) {
        pick_ability(duel, button.index);
        return true;
    }
    if find_in_lineage(entity, &sheet.pager, parents).is_some() {
        turn_the_page(duel);
        return true;
    }
    false
}

/// A click inside the zone browser.
///
/// Its own function rather than five more arms in [`pointer`]: they are one
/// widget, and the browser is meant to be a second *place* to click a card,
/// not a second way to answer a choice — which is why a tray card goes
/// through the same [`activate_card`] a card on the table does. *Opening* it
/// is not in here at all: that is a tap on the table, which reaches
/// [`open_pile`] through the ordinary card path.
///
/// Returns whether the click belonged to the browser.
pub(super) fn browser_click(
    duel: &mut Duel,
    entity: Entity,
    tray: &mut TrayWidgets,
    parents: &Query<&ChildOf>,
) -> bool {
    if let Some(card) = find_in_lineage(entity, &tray.cards, parents) {
        activate_card(duel, card.object);
        return true;
    }
    // The end of a pile: the held card goes last in it. Drawn only while
    // that would work, so a refusal here is a sheet one frame behind the
    // model, and the next frame draws it right.
    if let Some(slot) = find_in_lineage(entity, &tray.slots, parents) {
        if let Some(it) = duel.interaction.as_mut() {
            it.place_held(slot.0);
        }
        return true;
    }
    // A tab inside the open tray ticks a zone's box; a chip outside it opens
    // and closes the whole panel. Two different jobs, so two components.
    //
    // "Alle" is the one chip that is not a box being ticked — it is every box
    // being cleared, which is the same state and is why `Browser` keeps one
    // set and not a set plus a flag. Everything else toggles, so a second
    // click on a pile takes it back out of the merge.
    if let Some(tab) = find_in_lineage(entity, &tray.tabs, parents) {
        match tab.zone {
            Some(zone) => duel.browser.tick(zone),
            None => duel.browser.show(None),
        }
        return true;
    }
    if find_in_lineage(entity, &tray.close, parents).is_some() {
        duel.browser.toggle_by_hand();
        return true;
    }
    // The same call from the other end. The button on the ledge is the only
    // one of the two that is drawn while the sheet is *down*, which is what
    // makes "minimised" a true word for the state `close` writes.
    if find_in_lineage(entity, &tray.zones, parents).is_some() {
        duel.browser.toggle_by_hand();
        return true;
    }
    // Out to the band, or back to where it was. It is a toggle over one
    // question — `is_maximised` — rather than a remembered flag, because the
    // window can be resized under a maximised sheet and a flag would then be
    // saying something the rectangle does not.
    //
    // `duel.tray_restore` is the other half and is *not* the store: what the
    // sheet goes back to is the last rectangle the player arranged, and the
    // store now holds the band. Nothing restores to a placement nobody chose,
    // which is why an empty `tray_restore` restores to centred rather than to
    // whatever `settings.zone_browser` last was.
    if find_in_lineage(entity, &tray.grow, parents).is_some() {
        let band = crate::hud::band_of(&tray.windows);
        let now = tray
            .settings
            .zone_browser
            .map_or_else(|| Placement::centred(band), |p| p.fit(band));
        let next = if now.is_maximised(band) {
            duel.tray_restore
                .take()
                .map_or_else(|| Placement::centred(band), |p| p.fit(band))
        } else {
            duel.tray_restore = Some(now);
            Placement::maximised(band)
        };
        tray.settings.zone_browser = Some(next);
        tray.settings.save();
        *tray.glide = TrayGlide(Some(Flight {
            from: now,
            to: next,
            t: 0.0,
        }));
        return true;
    }
    // The dialog's way out, which exists only when the question's minimum is
    // zero. It sends the **empty** answer and not the assembled one: a player
    // who ticked a card and then changed their mind must not have that card
    // sent under the word "Cancel", so the answer is cleared first and
    // confirmed after. There is no cancel on the wire; the two together are
    // the closest thing to one there is.
    if find_in_lineage(entity, &tray.cancel, parents).is_some() {
        if let Some(it) = duel.interaction.as_mut() {
            it.cancel();
        }
        if let Some(action) = duel.interaction.as_ref().and_then(Interaction::confirm) {
            duel.submit(action);
        }
        return true;
    }
    if let Some(sort) = find_in_lineage(entity, &tray.sort, parents) {
        if sort.reverse {
            duel.browser.reverse();
        } else {
            duel.browser.cycle_sort();
        }
        return true;
    }
    // The one control on this panel that writes to the settings store rather
    // than to the `Browser`: which shape the list is drawn in is not a fact
    // about the game, and a player who picked the grid once should not have
    // to pick it again next launch. Saved on the click and not on a timer —
    // it is one small file and a click is the moment the player decided.
    if let Some(view) = find_in_lineage(entity, &tray.views, parents) {
        let mode = view.mode;
        if tray.settings.zone_view != mode {
            tray.settings.zone_view = mode;
            tray.settings.save();
        }
        return true;
    }
    // The builder's own buttons, before the box they sit under: the panel is
    // inside the same head as the filter row, and a row's text box is a
    // `Button` of its own.
    if let Some(act) = find_in_lineage(entity, &tray.acts, parents) {
        let act = act.0;
        duel.browser.filter_act(act);
        // *Done* is both: the act gives the row's caret back, and the marker
        // beside it shuts the panel. Read after the act rather than instead
        // of it, so a row still holding the caret does not keep it in a
        // builder nobody can see.
        if find_in_lineage(entity, &tray.done, parents).is_some() {
            duel.browser.close_builder();
        }
        return true;
    }
    // The gear opens the builder on what the box holds, and shuts it again.
    if find_in_lineage(entity, &tray.gear, parents).is_some() {
        duel.browser.toggle_builder();
        return true;
    }
    // The filter box takes the keyboard on the click and gives it back on
    // the next one, so a player can leave the panel open and keep playing.
    // A click here also shuts the builder: the box and the builder are two
    // editors of one string, and only one of them may hold the caret.
    if find_in_lineage(entity, &tray.filter, parents).is_some() {
        duel.browser.close_builder();
        if duel.browser.is_typing() {
            duel.browser.stop_typing();
        } else {
            duel.browser.start_typing();
        }
        return true;
    }
    false
}

/// Whether a click is the pointer's `ActivateGroup`: the whole merged card.
///
/// Read raw, like the shift that turns a preview over (`flip::turn`): a
/// modifier held under a pointer gesture is no chord the keymap could bind,
/// and no text field is reading the click. An app with no keyboard — a touch
/// screen, a test harness — has no shift.
pub(super) fn shift_held(keys: Option<&ButtonInput<KeyCode>>) -> bool {
    keys.is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]))
}
