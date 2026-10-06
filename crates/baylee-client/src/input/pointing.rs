//! The pointer: presses, hover, navigating the camera.

#[allow(clippy::wildcard_imports)] // the input module's shared vocabulary
use super::*;

/// Pointer handling: clicking a card, a player tab, or a rail button.
///
/// A click means "this object", and what that does depends entirely on the
/// pending choice: it selects a target, declares an attacker, or plays a card.
/// Resolving that here rather than in the renderer keeps one place where a
/// click becomes an action.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one branch per clickable widget kind
pub fn pointer(
    mut clicks: MessageReader<Pointer<Click>>,
    cards: Query<&CardVisual>,
    hand_cards: Query<&HandCardVisual>,
    tabs: Query<&PlayerTab>,
    seat_steps: Query<&crate::hud::SeatStep>,
    menu_buttons: Query<&MenuButton>,
    prompt_buttons: Query<&PromptButton>,
    sheet: SheetWidgets,
    choice_buttons: Query<&ChoiceButton>,
    mut tray: TrayWidgets,
    parents: Query<&ChildOf>,
    mut duel: ResMut<Duel>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut rig: ResMut<crate::table::CameraRig>,
    mut touched: ResMut<crate::touch::Touched>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
) {
    let whole = shift_held(keys.as_deref());
    for click in clicks.read() {
        let e = click.entity;
        // Every click disarms the concession, and the concede branch below
        // reads what it *was*. Taken here rather than in that branch so the
        // rule is one line and cannot be forgotten by a widget added later:
        // a half-pressed concession survives exactly nothing.
        let was_armed = std::mem::take(&mut duel.concede_armed);
        if let Some(object) = find_in_lineage(e, &cards, &parents)
            .map(|v| v.object)
            .or_else(|| find_in_lineage(e, &hand_cards, &parents).map(|h| h.object))
        {
            // The card under the finger is answered here and nowhere else:
            // this is the branch that knows both which card was tapped and
            // what the tap did, and a hand card that gave way under the press
            // has to be told which way to come back. A card on the *table*
            // wears no touch, and the call is harmless there because the
            // release has already taken the finger off nothing.
            let answer = activate(&mut duel, object, whole);
            crate::touch::answer(&mut touched, answer);
            continue;
        }
        if let Some(tab) = find_in_lineage(e, &tabs, &parents) {
            // A seat is a legal target of what is being cast ("any target",
            // CR 115.4), so the tab is how a player points at a face. It only
            // stops being a camera control while that is true — and a click
            // past `max` is swallowed rather than moving the camera, because
            // the player was aiming at a face and missing by one is not a
            // request to look somewhere else. What it still lacks is the line
            // saying so; that arrives with the slip.
            if let Some(i) = duel.interaction.as_mut()
                && (i.toggle_player(tab.player) != SelectionOutcome::Rejected
                    || matches!(
                        i.pending(),
                        baylee_engine::choice::Pending::ChooseTargets { .. }
                    ))
            {
                continue;
            }
            // Your own tab (or the already-focused one) brings the camera
            // home; any other opponent's tab frames their pod.
            if duel.seat() == Some(tab.player) || duel.focus == Some(tab.player) {
                navigate_home(&mut duel, &mut rig);
            } else {
                navigate_to_player(&mut duel, &mut rig, tab.player);
            }
            continue;
        }
        // A step tile on a seat bar toggles a standing order. `PhaseOrders`
        // is keyed by `RailSide` and not by seat, so an order about
        // opponents' turns is one order however many opponents are sitting at
        // the table — which is why one tile on one seat's bar can set an
        // order every other seat's bar then draws.
        if let Some(tile) = find_in_lineage(e, &seat_steps, &parents) {
            prefs.edit().orders.toggle(tile.side, tile.row);
            continue;
        }
        if let Some(button) = find_in_lineage(e, &menu_buttons, &parents) {
            menu_click(&mut duel, button.action, was_armed);
            continue;
        }
        if sheet_click(&mut duel, e, &sheet, &parents) {
            continue;
        }
        if let Some(button) = find_in_lineage(e, &choice_buttons, &parents) {
            if button.decision_id != duel.interaction.as_ref().and_then(Interaction::decision_id) {
                continue;
            }
            pick_choice(&mut duel, button.index);
            continue;
        }
        if browser_click(&mut duel, e, &mut tray, &parents) {
            continue;
        }
        if let Some(button) = find_in_lineage(e, &prompt_buttons, &parents) {
            if !button
                .matches_decision_id(duel.interaction.as_ref().and_then(Interaction::decision_id))
            {
                continue;
            }
            let action = match button.action {
                PromptAction::Yes => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_yes_no(true)),
                PromptAction::YesBatch => {
                    if let Some(batch) = duel
                        .view
                        .as_ref()
                        .zip(duel.interaction.as_ref())
                        .and_then(|(v, i)| crate::yes_batch::YesBatch::begin(v, i.pending()))
                    {
                        duel.yes_batch = batch;
                    }
                    None
                }
                PromptAction::No => {
                    duel.yes_batch = crate::yes_batch::YesBatch::default();
                    duel.interaction
                        .as_ref()
                        .and_then(|i| i.answer_yes_no(false))
                }
                PromptAction::Keep => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_mulligan(true)),
                PromptAction::Mulligan => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_mulligan(false)),
                // In a payment window confirm pays: the lands still owed
                // are tapped and the window settled after the last tap.
                PromptAction::Confirm if duel.pay_owed() => None,
                PromptAction::Confirm => {
                    let answer = committed_answer(&duel);
                    if answer.is_none() {
                        say_why_held_back(&mut duel);
                    }
                    answer
                }
                PromptAction::TargetBatch => duel
                    .interaction
                    .as_ref()
                    .zip(duel.view.as_ref())
                    .and_then(|(i, v)| baylee_client_core::targeting::batch_answer(i, v)),
                PromptAction::AutoDamage => {
                    duel.combat_auto =
                        duel.view
                            .as_ref()
                            .zip(duel.interaction.as_ref())
                            .and_then(|(v, i)| {
                                baylee_client_core::combat_auto::CombatAuto::begin(v, i.pending())
                            });
                    if duel.combat_auto.is_some() {
                        duel.stack_stop_requested = false;
                    }
                    None
                }
                // Aiming changes nothing the engine can hear; it moves the
                // focus the next declaration will use.
                PromptAction::AimNext => {
                    cycle_combat_focus(&mut duel, 1);
                    None
                }
                PromptAction::DeclareNothing => {
                    declare_nothing(&mut duel);
                    None
                }
                // Engaging the autopilot is not an answer either: it is a
                // standing instruction, and the pass it makes goes through
                // the same re-check against the current `LegalActions` that
                // the key does.
                PromptAction::SkipTurn => {
                    if let Some(turn) = duel.view.as_ref().map(|v| v.turn) {
                        duel.autopilot = Some(AutoPilot::ToNextTurn { from_turn: turn });
                    }
                    None
                }
                // Stepping changes nothing the engine can hear either: the
                // number is not an answer until Confirm sends it.
                PromptAction::Step(delta) => {
                    step_number(&mut duel, delta);
                    None
                }
            };
            if let Some(action) = action {
                duel.submit(action);
            }
            continue;
        }
        // A click on nothing interactive closes the card preview.
        duel.hovered = None;
        duel.hovered_at = None;
    }

    // The tap nobody heard, sent here — `docs/observed-faults.md` 35.
    //
    // Bevy raises a `Pointer<Click>` only when the press and the release land
    // on the same **entity**, and this hand row is rebuilt on every hover
    // change and on every view that arrives. A finger that is down while any
    // of that happens comes up on a node born after the press, no click is
    // raised, and the tap the player made simply did not happen: the card
    // gave way under the finger, came back, and played nothing. A view
    // arrives on every engine message, so this is not exotic — and the same
    // shape covers a press that drifts from a card's art onto its text, which
    // is two entities on one card and no rebuild at all.
    //
    // After the loop and not before it, which is the whole of the ordering:
    // an ordinary tap raises its click on the same frame as the release, and
    // that click clears the flag as it answers. Reading first would send
    // every tap twice — a land played and played again, a deed armed and then
    // fired.
    if let Some(object) = touched.swallowed_tap() {
        // What the loop takes from every click, taken once here for the same
        // reason: a half-pressed concession survives no tap either.
        duel.concede_armed = false;
        let answer = activate_card(&mut duel, object);
        crate::touch::answer(&mut touched, answer);
    }
}

/// Tracks the card under the pointer — the same cursor the WASD keys
/// move, so mouse and keyboard never fight over two highlights.
/// Like clicks, hover resolves through the entity's ancestors: the
/// card's image covers the whole card, and the event lands on it first.
///
/// # Why a still pointer says nothing
///
/// `Pointer<Over>` fires when the *card* moves under the pointer just as
/// readily as when the pointer moves over the card, and on this table the
/// cards are always moving: a repacked lane, a tap, a hover lift, a permanent
/// arriving. A pointer resting anywhere near the board therefore re-pinned
/// `hovered` every few frames and the keyboard cursor could not walk at all —
/// twelve presses moved it once, measured at a live table, which broke the
/// keymap's promise that every choice is answerable without a pointer. So the
/// pointer only speaks when it has actually moved. The grace of a few frames
/// covers the gap between a `CursorMoved` and the picking pass that follows
/// it, so a genuine mouse move is never swallowed.
#[allow(clippy::too_many_arguments)] // three picking queries plus the two event streams
pub fn pointer_hover(
    mut overs: MessageReader<Pointer<Over>>,
    mut outs: MessageReader<Pointer<Out>>,
    mut moves: MessageReader<bevy::window::CursorMoved>,
    mut grace: Local<u8>,
    mut source: Local<HoverSource>,
    mut zone: Local<Option<HoverZone>>,
    mut last: Local<Option<ObjectId>>,
    cards: Query<&CardVisual>,
    hand_cards: Query<&HandCardVisual>,
    tray_cards: Query<&TrayCard>,
    choice_previews: Query<&crate::hud::ChoicePreview>,
    piles: Query<&crate::table::PileVisual>,
    parents: Query<&ChildOf>,
    places: Query<&GlobalTransform>,
    table_camera: Query<(&Camera, &GlobalTransform), With<crate::table::TableCamera>>,
    mut duel: ResMut<Duel>,
) {
    // `hovered` has four writers and only one of them is this system. A hover
    // this system did not write knows nothing about where it came from, so it
    // is `Elsewhere` — which is the permissive answer, and has to be: the
    // keyboard cursor walking off a permanent and onto a hand card would
    // otherwise meet a stale `Table` source and be cleared on the next frame.
    // That is the stall of "The pointer only speaks when it moves" all over
    // again, through a different door.
    if duel.hovered != *last {
        *source = HoverSource::Elsewhere;
        *zone = None;
    }

    // A hovered card can leave without ever firing an `Out`, and playing the
    // card under the pointer is the ordinary way into that: the hand zone is
    // rebuilt, the node the pointer was over is despawned, and a despawned
    // entity reports nothing. So the hover is also held against the kind of
    // entity that reported it. A land played from the hand is no longer *a
    // hand card* whatever the battlefield now draws under the same id, which
    // is why the source matters and a bare "does this object still exist"
    // would not have cleared it.
    //
    // Checked before the grace window, because the pointer has not moved —
    // that is the whole point.
    if let Some(object) = duel.hovered {
        let alive = match *source {
            HoverSource::Hand => hand_cards.iter().any(|h| h.object == object),
            // Drawn *and* still lying where the pointer found it. The second
            // half is [`HoverZone`]'s whole reason for existing: a permanent
            // that becomes the top of a graveyard keeps this very entity and
            // stays a `CardVisual`, so "is it drawn" answers yes about a card
            // that has crossed the table.
            HoverSource::Table => {
                cards.iter().any(|v| v.object == object)
                    && duel.view.as_ref().is_none_or(|view| {
                        zone.is_none_or(|was| hover_zone(view, object) == Some(was))
                    })
            }
            // A tray row lives and dies with the panel: closing the browser,
            // switching its tab or typing into its filter rebuilds the whole
            // grid, and the row the pointer was over is gone without ever
            // firing an `Out`. Held against the tray's own rows for exactly
            // the reason the two above are held against theirs.
            HoverSource::Tray => tray_cards.iter().any(|t| t.object == object),
            HoverSource::Choice => choice_previews.iter().any(|t| t.object == object),
            // Somebody else's write — the keyboard cursor, most often, and
            // the union is that cursor's own invariant rather than a
            // weakening of the two above. The two kinds of hover are valid
            // for different reasons: a *pointer* hover holds while the
            // pointer is over the entity that reported it, and a *keyboard*
            // cursor holds while its object is anywhere in `cursor_grid`,
            // which spans the hand and every pod's lanes. An `ObjectId`
            // survives a zone change — nothing bumps a generation and
            // nothing frees an arena slot — so a card played off the cursor
            // is still in the grid, one row down, and `move_cursor` keeps
            // navigating from it. Clearing it there would not fix a ghost;
            // it would drop the player's cursor and send the next arrow key
            // back to the first card in hand.
            // `move_cursor` heals a stale one on its own, so only a card that
            // has left both places is cleared.
            HoverSource::Elsewhere => {
                hand_cards.iter().any(|h| h.object == object)
                    || cards.iter().any(|v| v.object == object)
            }
        };
        if !alive {
            duel.hovered = None;
            duel.hovered_at = None;
            *source = HoverSource::Elsewhere;
            *zone = None;
        }
    }

    if moves.read().next().is_some() {
        *grace = 3;
    }
    // Not an early return, because the last line has to run on every path:
    // a `*last` left behind would make this system read its own write as
    // somebody else's on the very next frame.
    if *grace == 0 {
        // Drain, so a later real move does not act on a backlog of events
        // the cards generated by sliding around.
        overs.clear();
        outs.clear();
    } else {
        *grace -= 1;
        for over in overs.read() {
            // Where the pointer was when it found the card, so the preview
            // can stand beside it. The event carries it; asking the window
            // for the cursor instead would answer with wherever the pointer
            // has since travelled, which on a fast sweep is a card or two
            // further along.
            let at = over.pointer_location.position;
            // The pointer is on one thing, so entering anything at all ends
            // whatever place it was on. Cleared here and written back a few
            // lines down when the thing entered is itself a place: without
            // it, walking off a library and onto a graveyard card would hold
            // both fans open, because a place reports its own `Out` a frame
            // later than the card reports its `Over`.
            duel.hovered_pile = None;
            if let Some((card, v)) = lineage_bearer(over.entity, &cards, &parents) {
                duel.hovered = Some(v.object);
                // The card itself, when it can be projected: a permanent on
                // the felt is a quad with a place in the world, so the
                // preview can stand at its *edge* rather than at the rim the
                // pointer crossed to get there — which is inside the card.
                duel.hovered_at = Some(
                    card_on_screen(card, &places, &table_camera)
                        .map_or(HoverSpot::Point(at), HoverSpot::Card),
                );
                *source = HoverSource::Table;
                // The place the claim is about, taken with the claim. A view
                // that has not arrived yet leaves it `None`, which holds the
                // hover rather than clearing it — the first view to name the
                // card fixes the zone on the next frame.
                *zone = duel
                    .view
                    .as_ref()
                    .and_then(|view| hover_zone(view, v.object));
            } else if let Some(h) = find_in_lineage(over.entity, &hand_cards, &parents) {
                duel.hovered = Some(h.object);
                duel.hovered_at = Some(HoverSpot::Point(at));
                *source = HoverSource::Hand;
            } else if let Some(t) = find_in_lineage(over.entity, &tray_cards, &parents) {
                // A row in the zone browser. It is a card drawn 74 px across,
                // which is enough to pick out and not enough to read, so it
                // previews like every other card the pointer finds — the
                // panel standing beside the pointer, because a tray row has
                // no place of its own in the HUD's layout.
                duel.hovered = Some(t.object);
                duel.hovered_at = Some(HoverSpot::Point(at));
                *source = HoverSource::Tray;
            } else if let Some(t) = find_in_lineage(over.entity, &choice_previews, &parents) {
                duel.hovered = Some(t.object);
                duel.hovered_at = Some(HoverSpot::Point(at));
                *source = HoverSource::Choice;
            } else if let Some(pile) = find_in_lineage(over.entity, &piles, &parents) {
                // A *place* rather than a card, which in practice means a
                // library. It writes neither `hovered` nor `hovered_at`:
                // there is no card here to look at, and a pile that opened
                // the preview panel would be claiming there is.
                duel.hovered_pile = Some((pile.player, pile.kind));
            }
        }
        for out in outs.read() {
            let is_current = find_in_lineage(out.entity, &cards, &parents)
                .is_some_and(|v| duel.hovered == Some(v.object))
                || find_in_lineage(out.entity, &hand_cards, &parents)
                    .is_some_and(|h| duel.hovered == Some(h.object))
                || find_in_lineage(out.entity, &tray_cards, &parents)
                    .is_some_and(|t| duel.hovered == Some(t.object))
                || find_in_lineage(out.entity, &choice_previews, &parents)
                    .is_some_and(|t| duel.hovered == Some(t.object));
            if is_current {
                duel.hovered = None;
                duel.hovered_at = None;
                *source = HoverSource::Elsewhere;
            }
            if find_in_lineage(out.entity, &piles, &parents)
                .is_some_and(|pile| duel.hovered_pile == Some((pile.player, pile.kind)))
            {
                duel.hovered_pile = None;
            }
        }
    }

    // The same staleness a hovered card is held against, for a place. A
    // library's slabs are despawned and rebuilt whenever its seat is — the
    // last card of a deck leaving is exactly that — and a despawned entity
    // fires no `Out`, so a fan left standing over a library that no longer
    // exists would never come down.
    if let Some((player, kind)) = duel.hovered_pile
        && !piles.iter().any(|p| p.player == player && p.kind == kind)
    {
        duel.hovered_pile = None;
    }

    *last = duel.hovered;
}

/// Which kind of entity reported the hover the client is currently drawing.
///
/// The pointer is the authority on *what* is hovered and the events are the
/// authority on when it stops — except that a card can be despawned out from
/// under a still pointer, which fires no event at all. This is what makes
/// that case answerable: the hover survives only while something of the same
/// kind still draws the object.
///
/// That is the whole answer for every kind but [`HoverSource::Table`], and
/// half of it there: a card on the table keeps its entity when it changes
/// zone, so the kind still answers yes about a permanent that is now the top
/// of a graveyard. [`HoverZone`] is the other half, and holds a table hover
/// against the *place* the pointer found the card in.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum HoverSource {
    /// A hand card, or a card in the command zone.
    Hand,
    /// A permanent on the table.
    Table,
    /// A row in the zone browser.
    Tray,
    /// A card named by a target or attacker choice row.
    Choice,
    /// The keyboard cursor, or nothing at all.
    #[default]
    Elsewhere,
}

/// Which zone a card the pointer found was lying in at the time.
///
/// A pointer hover is a claim about a **place**: the player put the pointer
/// on a card lying somewhere on the table. Nothing else in this system can
/// make that claim false when the card is the thing that moves.
/// `Pointer<Out>` does not fire, because the events are drained while the
/// pointer is still — that is "Why a still pointer says nothing", and it is
/// deliberate. [`HoverSource`]'s kind check does not fire either, because a
/// card that changes zone keeps its entity: `SceneIndex::cards` reuses it for
/// the pile top it becomes, so it is still drawn and still a `CardVisual`.
///
/// The card therefore glides out from under the pointer and takes the hover
/// with it. A fetchland sacrificed under the pointer is the everyday case,
/// and it cost a game: the hover followed Marsh Flats into the graveyard, and
/// [`the_click`] answers a hover before it answers anything else, so the
/// `Enter` that was meant to confirm the search the fetchland had just opened
/// opened the *graveyard* instead.
///
/// Only [`HoverSource::Table`] is held against this. A hand card leaving the
/// hand is already answered by its own kind check, and the keyboard cursor
/// (`Elsewhere`) is *meant* to follow a card across a zone — `move_cursor`'s
/// grid spans the hand and every lane, and clearing it there would drop the
/// player's cursor rather than fix a ghost.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HoverZone {
    /// A permanent on the battlefield.
    Battlefield,
    /// A spell or ability waiting to resolve.
    Stack,
    /// Somebody's graveyard.
    Graveyard,
    /// Exile.
    Exile,
    /// A command zone.
    Command,
    /// A card the game is showing everyone, and nowhere else.
    Shown,
}

/// Where the view says a card is, read in the order
/// [`baylee_view::PlayerView::object`] reads it, so the two can never
/// disagree about a card that is in two places at once.
///
/// `None` for a card the view does not carry — a card in hand, or one that
/// has left the game. A hover is never cleared on `None`, because a view that
/// has not arrived yet would otherwise take the pointer's answer away.
fn hover_zone(view: &baylee_view::PlayerView, object: ObjectId) -> Option<HoverZone> {
    let is_it = |o: &baylee_view::PublicObject| o.id == object;
    if view.battlefield.iter().any(is_it) {
        return Some(HoverZone::Battlefield);
    }
    if view.stack.iter().any(is_it) {
        return Some(HoverZone::Stack);
    }
    if view.graveyards.iter().flatten().any(is_it) {
        return Some(HoverZone::Graveyard);
    }
    if view.exile.iter().flatten().any(is_it) {
        return Some(HoverZone::Exile);
    }
    if view.command.iter().flatten().any(is_it) {
        return Some(HoverZone::Command);
    }
    view.looking_at
        .iter()
        .any(is_it)
        .then_some(HoverZone::Shown)
}

/// Navigates the camera to a seat.s pod, framing it in the free canvas
/// area (clear of the tab strip and the hand zone), cards upright. Also
/// marks the seat as the layout.s focus so its pod is enlarged.
pub fn navigate_to_player(
    duel: &mut Duel,
    rig: &mut crate::table::CameraRig,
    player: baylee_core::ids::PlayerId,
) {
    let Some(slot) = duel.layout.as_ref().and_then(|l| l.slot(player).copied()) else {
        return;
    };
    let world = Vec2::new(slot.center.x, -slot.center.y);
    *rig = crate::table::CameraRig::framing(&slot, world);
    duel.focus = Some(player);
    // Aimed on purpose, so the table stops framing itself: this is the *one*
    // seat the player asked to look at, and re-framing the whole ring on the
    // next resize would take it away from them.
    duel.camera_held = true;
    crate::rebuild_board(duel);
}

/// Returns the camera to the view behind the local seat that takes in the
/// whole table.
///
/// The rig is set back to its default rather than to that framing, because
/// the framing depends on the window and on the layout this call is about to
/// change: `table::frame_table` recognises the default as "nobody aimed this"
/// and puts the table back in frame on the next tick.
pub fn navigate_home(duel: &mut Duel, rig: &mut crate::table::CameraRig) {
    *rig = crate::table::CameraRig::default();
    duel.focus = None;
    duel.camera_held = false;
    crate::rebuild_board(duel);
}

/// Stack navigation must not steal a pending target choice.
pub(super) fn select_stack_stop(duel: &mut Duel, object: ObjectId) -> bool {
    if duel
        .view
        .as_ref()
        .is_some_and(|view| view.stack.iter().any(|item| item.id == object))
        && !duel
            .interaction
            .as_ref()
            .is_some_and(|choice| choice.is_selectable(object))
    {
        duel.stack_selected = (duel.stack_selected != Some(object)).then_some(object);
        return true;
    }
    false
}
