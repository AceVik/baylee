//! Input: keyboard first, pointer second.
//!
//! `docs/keyboard-map.md` makes a commitment — every choice the game can ask is
//! answerable without a pointer, and nothing requires drag-and-drop. That is
//! not only an accessibility promise; a competitive player passing priority
//! forty times a turn will not reach for a mouse each time.
//!
//! Both paths converge on the same place: they build a [`PlayerAction`] through
//! [`baylee_client_core::interaction::Interaction`], which refuses anything the
//! engine did not offer. No input handler decides legality by itself.
//!
//! Which key does what is the account's, not this module's: every binding
//! comes from `Keymap` and is resolved through `crate::keys`. The primary key
//! (`Enter` by default) is "the click", with a fixed precedence: the card
//! under the cursor, then the selected phase button, then confirm/pass —
//! which is why it is not the pass key: `Space` is `Confirm` and passes
//! whatever the cursor happens to be resting on.

use crate::hud::{
    AbilityButton, ChoiceButton, HandCardVisual, MenuAction, MenuButton, PlayerTab, PreviewResize,
    PromptAction, PromptButton, TrayCancel, TrayCard, TrayClose, TrayFilter, TraySort, TrayTab,
};
use crate::keys::Fired;
use crate::settings::ClientSettings;
use crate::table::CardVisual;
use crate::{Deed, Duel, HoverSpot};
use baylee_client_core::abilitysheet;
use baylee_client_core::automation::AutoPilot;
use baylee_client_core::browser::Placement;
use baylee_client_core::interaction::{Interaction, Prompt, SelectionOutcome};
use baylee_client_core::prefs::Action;
use baylee_client_core::touch::Answer;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::PlayerAction;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

/// Every widget the zone browser puts on screen, as one system parameter.
///
/// Bundled rather than four more arguments because `pointer` already sits at
/// Bevy's parameter limit — and because they are one thing: the tray, its
/// tabs, its close button, its filter box and its sort control.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TrayWidgets<'w, 's> {
    cards: Query<'w, 's, &'static TrayCard>,
    tabs: Query<'w, 's, &'static TrayTab>,
    close: Query<'w, 's, &'static TrayClose>,
    sort: Query<'w, 's, &'static TraySort>,
    filter: Query<'w, 's, &'static TrayFilter>,
    cancel: Query<'w, 's, &'static TrayCancel>,
}

/// Finds a component on the clicked entity or one of its ancestors —
/// a click on a button's icon or text belongs to the button.
///
/// The query carries its own filter, which every caller but one leaves empty.
/// The exception is [`crate::touch`], and it is why the parameter is here:
/// [`HandCardVisual`] speaks for every card the *HUD* draws, the stack
/// panel's slots included, so a system that is about the hand **row** has to
/// say so — and the query is where saying it also narrows the walk.
pub(crate) fn find_in_lineage<'a, T: Component, F: bevy::ecs::query::QueryFilter>(
    entity: Entity,
    query: &'a Query<'_, '_, &T, F>,
    parents: &Query<&ChildOf>,
) -> Option<&'a T> {
    lineage_bearer(entity, query, parents).map(|(_, found)| found)
}

/// The same walk, answering *which* ancestor carried the component.
///
/// The entity is what a caller needs to ask a second question about the card
/// — where it is on the screen, for one, which is a `GlobalTransform` on that
/// same entity and not on whichever child the pointer happened to land on.
fn lineage_bearer<'a, T: Component, F: bevy::ecs::query::QueryFilter>(
    entity: Entity,
    query: &'a Query<'_, '_, &T, F>,
    parents: &Query<&ChildOf>,
) -> Option<(Entity, &'a T)> {
    let mut current = Some(entity);
    for _ in 0..6 {
        let e = current?;
        if let Ok(found) = query.get(e) {
            return Some((e, found));
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}

/// Where a card on the felt lies on the screen, in logical pixels.
///
/// The four corners of its printed face, projected and boxed — not the
/// centre and a guess at a width. A card is a quad on a table seen at
/// `CAMERA_LEAN`, so what it covers on the screen depends on where at the
/// table it is and on whether it is tapped, and both of those are already in
/// its `GlobalTransform`. The mesh is built in local XY with the face on +Z
/// ([`rounded_slab_mesh`](crate::table)), so the corners are
/// `(±width/2, ±height/2, 0)` whatever the transform then does with them.
///
/// `None` when there is no table camera yet, when the entity has no place, or
/// when any corner projects behind the lens — a partly visible box is worse
/// than none, because the panel would open at an edge that is not the card's.
fn card_on_screen(
    card: Entity,
    places: &Query<&GlobalTransform>,
    camera: &Query<(&Camera, &GlobalTransform), With<crate::table::TableCamera>>,
) -> Option<Rect> {
    use baylee_client_core::layout::{CARD_HEIGHT, CARD_WIDTH};

    let (cam, eye) = camera.iter().next()?;
    let place = places.get(card).ok()?;
    let (hw, hh) = (CARD_WIDTH / 2.0, CARD_HEIGHT / 2.0);
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for (x, y) in [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)] {
        let at = cam
            .world_to_viewport(eye, place.transform_point(Vec3::new(x, y, 0.0)))
            .ok()?;
        min = min.min(at);
        max = max.max(at);
    }
    Some(Rect { min, max })
}

/// Whether the card `entity` belongs to is still gliding towards its mark.
///
/// Asked of an `Out`, and the hover lift is why. Hovering raises a card by
/// `HOVER_LIFT` and grows it by `HOVER_SCALE`; the growth is self-correcting,
/// the rise is not, so a pointer resting near a card's lower edge ends up
/// outside it a few frames after the hover starts. Bevy fires `Out`, the hover
/// clears, the card falls back, `Over` fires, and the card blinks for as long
/// as the pointer stays where it is.
///
/// The `grace` window below already drops events from cards sliding around —
/// but only while the pointer is *still*, and it is rearmed by every
/// `CursorMoved`. A player moving the mouse slowly across their own lands
/// therefore holds it permanently open, which is precisely the case this was
/// reported in.
///
/// So: a card that has not finished moving cannot testify that the pointer
/// went anywhere, because it is the thing that moved. A settled card's `Out`
/// is evidence and is honoured. `glide` snaps the transform onto its target
/// once inside `SETTLED`, so the comparison is exact rather than a second
/// threshold that could disagree with the first.
/// The one way a card becomes an action: play it when the engine offers
/// that, otherwise select it for the pending choice. Clicks and the
/// keyboard cursor both end here, so they can never disagree.
///
/// Between those two there is now a third answer. A spell the engine has not
/// offered because the mana is not floating yet is not a card with nothing to
/// do — it is a card that wants two lands tapped first, which is what a player
/// at a table would do without thinking about it. [`crate::mana_for`] works
/// out which lands; the run in [`crate::ManaRun`] taps them and then casts.
///
/// It answers whether the tap **did** anything, because the drawing needs to
/// know and this is the one place that can say. A tap nothing claimed is not
/// an error and is not rare — on an opponent's turn most of a hand is
/// sorceries — but it was indistinguishable from a dead button, and
/// reconstructing the answer from what changed in `Duel` would be a second
/// reading of these branches, kept in step with them by hand. See
/// [`baylee_client_core::touch`].
pub fn activate_card(duel: &mut Duel, object: ObjectId) -> Answer {
    // A second tap on the same card is the send. A tap on a different one is
    // a change of mind and not a confirmation, so it disarms and arms afresh.
    if duel.armed.as_ref().is_some_and(|a| a.object == object) {
        fire_armed(duel);
        return Answer::Took;
    }
    duel.armed = None;
    if let Some(action) = duel.interaction.as_ref().and_then(|i| i.play_card(object)) {
        // A land plays on the click. See `one_click_land` below for the line
        // that lets it and for what stays on the far side of that line.
        if duel
            .interaction
            .as_ref()
            .is_some_and(|i| i.plays_only_as_a_land(object))
        {
            duel.submit(action);
        } else {
            arm(duel, object, Deed::Play);
        }
        return Answer::Took;
    }
    if duel.reachable.contains(&object)
        && let Some(plan) = crate::mana_for(duel, object)
    {
        duel.last_error = None;
        arm(
            duel,
            object,
            Deed::Run {
                plan,
                then: crate::RunEnd::Cast,
            },
        );
        return Answer::Took;
    }
    // Suspending is the fourth thing a card in hand can do, and it was the
    // one nothing could reach: `legal.suspendable` was read by the automation
    // rules and by nothing else, so a Suspend 4—{U} answered no click at all.
    // Both halves sit here in the order the two above do — the engine's own
    // offer first, this client's offer to tap for it second.
    //
    // *After* the cast, which is a decision and not an accident: a card that
    // could be cast and suspended on the same click is two deeds and this
    // path would silently pick one. No card in the pool is both — every
    // suspend card there prints no mana cost, so CR 202.1a keeps it out of
    // `castable` entirely — and `no_suspend_card_in_the_pool_is_also_castable`
    // is what says so, because the day one is, this line has to become a
    // chooser rather than an order.
    if duel
        .interaction
        .as_ref()
        .is_some_and(|i| i.suspend(object).is_some())
    {
        arm(duel, object, Deed::Suspend);
        return Answer::Took;
    }
    if duel.suspend_reach.contains(&object)
        && let Some(plan) = crate::suspend_mana_for(duel, object)
    {
        duel.last_error = None;
        arm(
            duel,
            object,
            Deed::Run {
                plan,
                then: crate::RunEnd::Suspend,
            },
        );
        return Answer::Took;
    }
    // A permanent with something to do does it. One ability goes straight
    // through — a menu of one only ever wastes a tap — and several open the
    // chooser in the prompt bar, because "ability 2" is not a thing a player
    // should have to count out on a card.
    if let Some(options) = abilities_of(duel, object) {
        match options.len() {
            0 => {}
            1 => {
                duel.ability_menu = None;
                arm_ability(duel, object, &options[0]);
                return Answer::Took;
            }
            _ => {
                duel.ability_menu = Some(object);
                duel.ability_pick = 0;
                duel.ability_page = 0;
                return Answer::Took;
            }
        }
    }
    duel.ability_menu = None;
    let answered = duel
        .interaction
        .as_mut()
        .is_some_and(|i| i.toggle(object) != SelectionOutcome::Rejected);
    if answered || open_pile(duel, object) {
        Answer::Took
    } else {
        Answer::Refused
    }
}

/// A tap that meant nothing else, on a card lying on top of a pile, opens
/// that pile.
///
/// Last of all the branches above, and that ordering is the rule rather than
/// an accident: while the engine is asking a player to choose a card out of
/// their graveyard, a tap on the top of it must *answer the question*, not
/// drop a panel over the board they are answering it from. Only a tap that
/// nothing else claimed is a request to look through the pile.
///
/// Answers whether it opened anything, which is the last word on whether the
/// tap did: everything above this has already said no.
fn open_pile(duel: &mut Duel, object: ObjectId) -> bool {
    let Some(board) = duel.board.as_ref() else {
        return false;
    };
    let opening = board.pods.iter().find_map(|pod| {
        pod.piles
            .iter()
            .find(|pile| pile.top == Some(object) && pile.is_browsable())
            .and_then(|pile| baylee_client_core::BrowseZone::of_pile(pile.kind, pod.player))
    });
    if let Some(zone) = opening {
        duel.browser.open_at(zone);
        return true;
    }
    false
}

/// Arms a deed. The prompt bar draws what is armed, and the way out of it.
fn arm(duel: &mut Duel, object: ObjectId, deed: Deed) {
    duel.armed = Some(crate::Armed { object, deed });
}

/// One ability: sent outright when the whole cost is this permanent's own
/// tap, armed otherwise.
///
/// # The one-click line
///
/// `docs/design.md` §2.5 had this as "mana abilities only", on the grounds
/// that floating mana is the cheap mistake. Playing the client made the
/// better statement of the same rule: an action is one click when **its whole
/// cost comes out of the card itself and the next untap step undoes it**, and
/// nothing else moves.
///
/// A mana ability passes that (and keeps its own CR 605.1 reason for being
/// one tap). So does `{T}: Draw a card` — the permanent is tapped, and being
/// tapped is over at the start of your next turn. Playing a land passes it
/// too, which is why [`activate_card`] sends one on the click: the worst case
/// is the wrong land in the one land drop.
///
/// What stays on the far side of the line is everything whose cost leaves the
/// card: a sacrifice, a discard, an exile, life, mana, and a loyalty ability,
/// whose counters no untap step gives back. Those are still arm-then-act,
/// because there is no undo in the engine and never will be.
///
/// Returns whether the ability was *sent*. The ability sheet stands open
/// while a row is armed — the roundel goes gilt and the same digit again
/// sends it — so the caller needs to know which of the two happened, and a
/// sheet that closed on the arming would take the digit away with it.
fn arm_ability(
    duel: &mut Duel,
    object: ObjectId,
    option: &crate::abilities::AbilityOption,
) -> bool {
    // Whether this row is armed already is never asked here: `fire_armed` is
    // the path for that, and it re-resolves the deed against the current
    // `LegalActions` rather than trusting a list drawn a frame ago.
    match abilitysheet::press(option.mana || option.tap_only, false) {
        abilitysheet::Press::Send => {
            duel.submit(option.action.clone());
            true
        }
        abilitysheet::Press::Arm | abilitysheet::Press::Fire => {
            arm(duel, object, Deed::Ability(option.action.clone()));
            false
        }
    }
}

/// Sends what is armed, or disarms when the engine no longer offers it.
///
/// Everything is resolved against the *current* `LegalActions` rather than
/// trusted from the tap that armed it — the rule the ability chooser and
/// [`crate::ManaRun`] both already follow. A deed that has gone stale leaves
/// nothing on the wire and says why.
pub fn fire_armed(duel: &mut Duel) {
    let Some(armed) = duel.armed.take() else {
        return;
    };
    // The ability sheet stands open while a row is armed, because the digit
    // that armed it is the digit that sends it. Once it is sent the sheet has
    // been answered, and a list of things to do left standing over a card
    // whose ability is already on the stack is a list a player has to dismiss.
    duel.ability_menu = None;
    match armed.deed {
        Deed::Play => {
            match duel
                .interaction
                .as_ref()
                .and_then(|i| i.play_card(armed.object))
            {
                Some(action) => duel.submit(action),
                None => duel.last_error = Some(STALE.to_string()),
            }
        }
        Deed::Ability(action) => {
            let still_offered = abilities_of(duel, armed.object)
                .is_some_and(|options| options.iter().any(|o| o.action == action));
            if still_offered {
                duel.submit(action);
            } else {
                duel.last_error = Some(STALE.to_string());
            }
        }
        // The plan itself is not re-planned: `ManaRun` re-checks every one of
        // its steps against what the engine is offering as it spends them, and
        // stops honestly if a land it counted on can no longer be tapped. What
        // *is* re-read is whether a run is still the right answer — between the
        // two taps this seat holds priority, so the one thing that can have
        // changed is its own manual land tap, after which the spell may be
        // castable outright and the run would float mana nobody asked for.
        Deed::Suspend => {
            match duel
                .interaction
                .as_ref()
                .and_then(|i| i.suspend(armed.object))
            {
                Some(action) => duel.submit(action),
                None => duel.last_error = Some(STALE.to_string()),
            }
        }
        Deed::Run {
            plan,
            then: crate::RunEnd::Cast,
        } => {
            if let Some(action) = duel
                .interaction
                .as_ref()
                .and_then(|i| i.play_card(armed.object))
            {
                duel.submit(action);
            } else if duel.reachable.contains(&armed.object) {
                duel.last_error = None;
                duel.mana_run = Some(crate::ManaRun::new(plan, armed.object, crate::RunEnd::Cast));
            } else {
                duel.last_error = Some(STALE.to_string());
            }
        }
        // The same shape for the other end, and the same short-circuit: the
        // manual tap that happened between the two clicks may already have
        // floated the cost, in which case the engine is offering the suspend
        // outright and a run would float mana nobody asked for.
        Deed::Run {
            plan,
            then: crate::RunEnd::Suspend,
        } => {
            if let Some(action) = duel
                .interaction
                .as_ref()
                .and_then(|i| i.suspend(armed.object))
            {
                duel.submit(action);
            } else if duel.suspend_reach.contains(&armed.object) {
                duel.last_error = None;
                duel.mana_run = Some(crate::ManaRun::new(
                    plan,
                    armed.object,
                    crate::RunEnd::Suspend,
                ));
            } else {
                duel.last_error = Some(STALE.to_string());
            }
        }
    }
}

/// What the bar says when an armed deed no longer exists.
///
/// English here, like the mana run's own abort lines beside it: `last_error`
/// is one channel carrying the gateway's words as well as the client's, and
/// translating half of it would be worse than translating none.
const STALE: &str = "the engine no longer offers that";

/// What `object` is offering, if anything.
/// English deliberately: only the `action` on each option is read here —
/// what is drawn is [`crate::hud`]'s business, and this path picks an ability
/// by position or takes the only one there is.
fn abilities_of(duel: &Duel, object: ObjectId) -> Option<Vec<crate::abilities::AbilityOption>> {
    let view = duel.view.as_ref()?;
    let interaction = duel.interaction.as_ref()?;
    Some(crate::abilities::options(
        baylee_client_core::Lang::En,
        view,
        interaction,
        object,
    ))
}

/// Keyboard handling: every key comes from the account's keymap.
///
/// The handler asks *actions*, never keys. That is what makes rebinding work
/// at all, and it also removed the `if !shift` guards that used to be sprayed
/// through here — `W` and `⇧W` are two chords, and telling them apart is the
/// keymap's job, not this function's.
pub fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    mut duel: ResMut<Duel>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut rig: ResMut<crate::table::CameraRig>,
    mut settings: ResMut<crate::settings::ClientSettings>,
) {
    let fired = Fired::of(&keys, prefs.keymap());
    // Before the quiet check, and not after it: a letter typed into the type
    // filter is usually bound to no action at all, so `Fired` is empty for
    // exactly the keys the box cares about most.
    if subtype_keys(fired, &mut typed, &mut duel) {
        return;
    }
    // Same reason, and the same place in the order: a digit is bound to no
    // action, so `Fired` is empty for exactly the keys a number choice wants.
    if number_keys(&mut typed, &mut duel) {
        return;
    }
    // And once more for the ability sheet, whose rows are sent by the digit
    // drawn on each of them. Same place in the order and the same reason: a
    // digit is bound to no action, so `Fired` is empty for exactly these
    // keys.
    if sheet_digits(&mut typed, &mut duel) {
        return;
    }
    // And the same again for the browser's filter box — but only while it has
    // been given the keyboard, because the panel can stand open for a whole
    // turn and a box that swallowed every keystroke would end playing with
    // the graveyard visible.
    if browser_keys(fired, &mut typed, &mut duel) {
        return;
    }
    if fired.quiet() {
        return;
    }
    // The keyboard's half of the same rule as the pointer's: any bound key
    // forgets a half-pressed concession.
    duel.concede_armed = false;
    look_around(fired, &mut duel, &mut rig, &mut settings, &mut prefs);
    // An armed deed owns the keyboard first, and ahead of the ability menu:
    // arming is where the chooser *ends*, so a confirm key reaching the menu
    // instead would pick a second ability rather than send the first.
    if armed_keys(fired, &mut duel) {
        return;
    }
    // The ability menu owns the keyboard while it stands: a list of things to
    // do is not a background for the cursor to walk over.
    if ability_menu_keys(fired, &mut duel) {
        return;
    }
    if move_the_cursor(fired, &mut duel) {
        return;
    }
    if aim_and_declare(fired, &mut duel) {
        return;
    }
    if the_click(fired, &mut duel, &mut prefs) {
        return;
    }
    if duel.interaction.is_some() {
        answer_the_question(fired, &mut duel, &mut prefs);
    }
}

/// Camera, phase rail, fast-forward and display toggles — everything that
/// changes what the player sees rather than what the game hears.
fn look_around(
    fired: Fired,
    duel: &mut Duel,
    rig: &mut crate::table::CameraRig,
    settings: &mut crate::settings::ClientSettings,
    prefs: &mut crate::prefs::Prefs,
) {
    if fired.has(Action::FocusNextSeat) {
        focus_next_opponent(duel, rig);
    }
    if fired.has(Action::FocusHome) {
        navigate_home(duel, rig);
    }
    // The rail: move the highlight here, toggle it with the primary key.
    if fired.has(Action::RailUp) {
        prefs.rail_cursor().move_selection(-1);
    }
    if fired.has(Action::RailDown) {
        prefs.rail_cursor().move_selection(1);
    }
    if let Some((phase, turn)) = duel.view.as_ref().map(|v| (v.phase, v.turn)) {
        if fired.has(Action::NextPhase) {
            duel.autopilot = Some(AutoPilot::ToNextPhase { from: phase });
        }
        if fired.has(Action::NextTurn) {
            duel.autopilot = Some(AutoPilot::ToNextTurn { from_turn: turn });
        }
    }
    if fired.has(Action::ToggleTextView) {
        // The modifier key shows the card face while held; this is the latch,
        // for players who read text rather than art. A preference, not a mode,
        // so it is remembered.
        settings.prefer_text_view = !settings.prefer_text_view;
        settings.save();
    }
    if fired.has(Action::ToggleBrowser) {
        // A latch rather than a held key, for the same reason a tap on a pile
        // opens one: reading a graveyard is not a glance, and a held key is
        // not a gesture a phone has. The tab it was last left on is kept, so
        // a player checking their own yard twice does not re-pick it.
        if duel.browser.is_open() {
            duel.browser.close();
        } else {
            duel.browser.open();
        }
    }
    // Here rather than beside the other answers, because a hold is the one
    // thing a seat says while it is *not* being asked: the engine takes a
    // `SetPriorityHold` from any seated player at any time, which is what
    // makes cancelling one possible at all. Both keys cancel a running hold
    // and only set one when none is; `Duel::hold_action` owns that rule.
    if (fired.has(Action::HoldForStack) || fired.has(Action::HoldForTurn))
        && let Some(action) = duel.hold_action(fired.has(Action::HoldForTurn))
    {
        duel.submit(action);
    }
}

/// Typing a number rather than stepping to it.
///
/// Stepping from 0 to 9 is nine presses, and X is routinely somebody's whole
/// hand of lands. So a digit types: it appends to what stands, and falls back
/// to the digit alone when appending would leave the offered range — which is
/// what a player means by typing `7` when the value already reads `12` and the
/// maximum is 9. Backspace takes a digit off, and the interaction clamps
/// whatever comes out, so nothing typed here is expressible outside the range
/// the engine offered.
/// The platform's own text input, pointed at the browser's filter box.
///
/// Only the browser has one, and it is the only thing that raises a phone's
/// keyboard — a canvas never does. The lobby's form does this for its fields;
/// the table has exactly one field, and without this the pile a player wanted
/// to search was searchable on a desktop and not on the device the tray's
/// scrolling and 44-pixel targets were sized for.
///
/// Focus is an *edge*: `typing_epoch` counts how many times the box has been
/// given the keyboard, because pointing the input at the field on every frame
/// would fight the player for the caret.
pub fn browser_softkeys(
    mut keys: ResMut<crate::softkeys::SoftKeyboard>,
    mut duel: ResMut<Duel>,
    mut epoch: Local<u64>,
    mut had_focus: Local<bool>,
) {
    if !crate::softkeys::SoftKeyboard::owns_typing() {
        return;
    }
    let typing = duel.browser.is_typing();
    if typing && *epoch != duel.browser.typing_epoch() {
        *epoch = duel.browser.typing_epoch();
        keys.open(
            baylee_client_core::lobby::FieldKind::Name,
            duel.browser.filter(),
        );
    }
    if !typing {
        if *had_focus {
            keys.close();
        }
        *had_focus = false;
        return;
    }
    *had_focus = true;
    for key in keys.drain() {
        match key {
            // Not a keystroke: autofill and paste arrive as a whole value.
            crate::softkeys::SoftKey::Text { value, .. } => duel.browser.set_filter(value),
            // The filter box is a string with no caret drawn in it, so a
            // caret that moved inside the element changes nothing here.
            crate::softkeys::SoftKey::Caret { .. } => {}
            // Nothing to submit — the rows are already narrowed, so the
            // action key means "done".
            crate::softkeys::SoftKey::Submit => {
                duel.browser.stop_typing();
                keys.close();
            }
            // Escape, arriving the long way round because the `<input>` has
            // the focus and the canvas never sees the key. Same two steps as
            // the native path in `browser_keys`: empty the box first, let go
            // of it second — `clear_filter` bumps the epoch, so the next
            // frame points the field at the emptied value rather than
            // leaving the old letters on screen.
            crate::softkeys::SoftKey::Dismiss => {
                if duel.browser.filter().is_empty() {
                    duel.browser.stop_typing();
                    keys.close();
                } else {
                    duel.browser.clear_filter();
                }
            }
        }
    }
}

/// Typing into the zone browser's filter box.
///
/// The panel could sort and scroll and the one thing the owner asked for by
/// name — a graveyard you can *search* — had no way in: `Browser::set_filter`
/// was written and nothing ever called it.
///
/// The box holds the keyboard only while it has been given it, which is what
/// lets the panel stay open through a turn. `Cancel` is the way out and
/// empties the box first when there is anything in it, so one press undoes
/// the search and the next one lets go — a player who typed `mou` and found
/// nothing should not have to rub out three letters to get back to the pile.
fn browser_keys(fired: Fired, typed: &mut MessageReader<KeyboardInput>, duel: &mut Duel) -> bool {
    if !duel.browser.is_typing() {
        return false;
    }
    // `Cancel` is read *before* the platform bail below, and that ordering is
    // the whole of it: where the browser does the typing every raw key
    // belongs to its `<input>`, so returning first would leave Escape doing
    // nothing at all on a page — not emptying the box, not letting go of it,
    // with only the soft keyboard's own action key as a way out. `fired`
    // carries actions rather than raw keys, so reading it here cannot type a
    // character the `<input>` has already taken.
    if fired.has(Action::Cancel) {
        if duel.browser.filter().is_empty() {
            duel.browser.stop_typing();
        } else {
            duel.browser.clear_filter();
        }
        return true;
    }
    // The box still owns the keyboard where the platform does the typing —
    // `browser_softkeys` has already read the value — but the client must not
    // read the raw keys as well, or every character is entered twice.
    if crate::softkeys::SoftKeyboard::owns_typing() {
        return true;
    }
    for event in typed.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Character(s) => {
                for c in s.chars() {
                    duel.browser.push_filter(c);
                }
            }
            Key::Space => duel.browser.push_filter(' '),
            Key::Backspace => {
                duel.browser.pop_filter();
            }
            // "Done" rather than "submit": the rows are already narrowed, so
            // the only thing left to do is hand the keyboard back.
            Key::Enter => duel.browser.stop_typing(),
            _ => {}
        }
    }
    true
}

fn number_keys(typed: &mut MessageReader<KeyboardInput>, duel: &mut Duel) -> bool {
    if !matches!(
        duel.interaction.as_ref().map(Interaction::prompt),
        Some(Prompt::ChooseNumber { .. })
    ) {
        return false;
    }
    let mut touched = false;
    for event in typed.read() {
        if !event.state.is_pressed() {
            continue;
        }
        let Some(i) = duel.interaction.as_mut() else {
            continue;
        };
        match &event.logical_key {
            Key::Character(s) => {
                for digit in s.chars().filter_map(|c| c.to_digit(10)) {
                    let appended = i.number().saturating_mul(10).saturating_add(digit);
                    // `set_number` clamps, so "did it fit" is asked by
                    // comparing what came back with what went in.
                    if i.set_number(appended) != appended {
                        i.set_number(digit);
                    }
                    touched = true;
                }
            }
            Key::Backspace => {
                let shorter = i.number() / 10;
                i.set_number(shorter);
                touched = true;
            }
            _ => {}
        }
    }
    touched
}

/// The creature types still on screen, in the engine's order.
fn visible_types(duel: &Duel) -> Vec<crate::choices::ChoiceOption> {
    duel.interaction
        .as_ref()
        .map(Interaction::prompt)
        .and_then(|p| {
            crate::choices::options(
                &p,
                baylee_client_core::Lang::En,
                duel.statics.as_ref(),
                &duel.subtype_filter,
            )
        })
        .unwrap_or_default()
}

/// Typing into the creature-type filter, and walking what it leaves.
///
/// While the box is up the keymap is swallowed whole, because letters *are*
/// chords: `W` walks the cursor and `E` activates a card, and a player
/// spelling "Elemental" would otherwise play half their turn. Only the keys
/// that mean something to a list survive — the cursor walks the rows, Confirm
/// takes the highlighted one, Cancel empties the box.
///
/// Returns whether it consumed the frame.
fn subtype_keys(fired: Fired, typed: &mut MessageReader<KeyboardInput>, duel: &mut Duel) -> bool {
    if !matches!(
        duel.interaction.as_ref().map(Interaction::prompt),
        Some(Prompt::ChooseSubtype { .. })
    ) {
        return false;
    }
    let before = duel.subtype_filter.clone();
    for event in typed.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Character(s) => duel
                .subtype_filter
                .extend(s.chars().filter(|c| !c.is_control())),
            Key::Backspace => {
                duel.subtype_filter.pop();
            }
            _ => {}
        }
    }
    let rows = visible_types(duel);
    if duel.subtype_filter != before {
        // The highlight follows the list. A row that has just been filtered
        // away must not stay picked, or Confirm answers a type the player can
        // no longer see.
        if let Some(first) = rows.first().map(|row| row.index)
            && let Some(i) = duel.interaction.as_mut()
        {
            i.choose_index(first);
        }
        return true;
    }
    if fired.has(Action::Cancel) {
        duel.subtype_filter.clear();
        return true;
    }
    let step = i32::from(fired.has(Action::CursorDown)) - i32::from(fired.has(Action::CursorUp))
        + i32::from(fired.has(Action::CursorRight))
        - i32::from(fired.has(Action::CursorLeft));
    let picked = duel
        .interaction
        .as_ref()
        .and_then(Interaction::chosen_index);
    if step != 0 && !rows.is_empty() {
        let at = picked
            .and_then(|p| rows.iter().position(|row| row.index == p))
            .and_then(|p| i32::try_from(p).ok())
            .unwrap_or(0);
        let len = i32::try_from(rows.len()).unwrap_or(1);
        let next = usize::try_from((at + step).rem_euclid(len)).unwrap_or(0);
        if let Some(row) = rows.get(next)
            && let Some(i) = duel.interaction.as_mut()
        {
            i.choose_index(row.index);
        }
        return true;
    }
    if (fired.has(Action::Confirm) || fired.has(Action::Primary))
        // Only a row that is still on screen: the filter may have moved on
        // since the highlight was set.
        && picked.is_some_and(|p| rows.iter().any(|row| row.index == p))
        && let Some(action) = duel.interaction.as_ref().and_then(Interaction::confirm)
    {
        duel.submit(action);
    }
    true
}

/// An armed deed: the confirm keys send it, cancel disarms. Returns whether
/// it consumed the frame.
///
/// Cancel is listed first in `docs/keyboard-map.md`'s Escape order for a
/// reason — an armed deed is the cheapest thing in the client to undo,
/// because it is the only one with nothing on the wire yet.
pub fn armed_keys(fired: Fired, duel: &mut Duel) -> bool {
    if duel.armed.is_none() {
        return false;
    }
    if fired.has(Action::Cancel) {
        duel.armed = None;
        return true;
    }
    if fired.has(Action::Primary) || fired.has(Action::Confirm) || fired.has(Action::ActivateCard) {
        fire_armed(duel);
        return true;
    }
    false
}

/// The open ability sheet: the cursor keys walk it, the primary key or
/// confirm takes the row, cancel puts the sheet away. Returns whether it
/// consumed the frame.
///
/// The list is rebuilt from `LegalActions` here rather than trusted from the
/// frame it was drawn on — the same rule the pointer path follows, and for
/// the same reason: the engine may have withdrawn the ability since.
///
/// The digits are not here. They are read as *characters* by
/// [`sheet_digits`], the way the subtype filter and a number entry are,
/// because a digit is bound to no [`Action`] and nine new ones would be nine
/// rows in every player's keymap for a key whose whole meaning is the number
/// printed on it.
pub fn ability_menu_keys(fired: Fired, duel: &mut Duel) -> bool {
    let Some(object) = duel.ability_menu else {
        return false;
    };
    let Some(options) = abilities_of(duel, object).filter(|o| o.len() > 1) else {
        // Nothing left to choose: the sheet is stale, and holding it open
        // would keep the keyboard hostage.
        duel.ability_menu = None;
        return false;
    };
    // The list can shrink under a page that was valid when it was turned to.
    duel.ability_page = abilitysheet::clamp(options.len(), duel.ability_page);
    if fired.has(Action::Cancel) {
        duel.ability_menu = None;
        return true;
    }
    let step = i32::from(fired.has(Action::CursorDown)) - i32::from(fired.has(Action::CursorUp))
        + i32::from(fired.has(Action::CursorRight))
        - i32::from(fired.has(Action::CursorLeft));
    if step != 0 {
        let len = i32::try_from(options.len()).unwrap_or(1);
        let next = i32::try_from(duel.ability_pick).unwrap_or(0) + step;
        duel.ability_pick = usize::try_from(next.rem_euclid(len)).unwrap_or(0);
        // The cursor walks the whole list and the sheet shows nine rows of
        // it, so walking off the end of a page turns it. Deriving the page
        // from the cursor rather than moving them separately is what stops
        // the highlight from being on a row that is not drawn.
        duel.ability_page = duel.ability_pick / abilitysheet::PAGE;
        return true;
    }
    if fired.has(Action::Primary) || fired.has(Action::Confirm) || fired.has(Action::ActivateCard) {
        if let Some(option) = options.get(duel.ability_pick).cloned()
            && arm_ability(duel, object, &option)
        {
            duel.ability_menu = None;
        }
        return true;
    }
    false
}

/// The digits, while the ability sheet stands: each row is sent by the number
/// drawn on it, and `0` turns the page. Returns whether it consumed the
/// frame.
///
/// Read straight off `KeyboardInput` as `Key::Character`, the way
/// `subtype_keys` and the number-entry path already read digits. The
/// alternative is nine new [`Action`]s, which is nine rows in every keymap
/// screen and a rebinding a player could make — for a key whose entire
/// meaning is the numeral printed beside the row.
///
/// A row with a cost **arms** on the first press and sends on the second,
/// which is [`abilitysheet::press`]'s whole answer; a mana ability and a
/// `{T}`-only ability send on the first, which is the same one-tap exemption
/// [`arm_ability`] applies everywhere else.
fn sheet_digits(typed: &mut MessageReader<KeyboardInput>, duel: &mut Duel) -> bool {
    // Both halves of the guard are about *not reading the events*. The sheet
    // can stand open while the browser's filter box has the keyboard, and
    // `typed.read()` drains every key rather than the digits alone — so a
    // sheet that read here unconditionally would eat the letters a player is
    // typing into that box and open an ability with the digits.
    if duel.ability_menu.is_none() || duel.browser.is_typing() {
        return false;
    }
    let pressed: Vec<char> = typed
        .read()
        .filter(|event| event.state.is_pressed())
        .filter_map(|event| match &event.logical_key {
            Key::Character(s) => Some(s.chars()),
            _ => None,
        })
        .flatten()
        .filter(char::is_ascii_digit)
        .collect();
    if pressed.is_empty() {
        return false;
    }
    let mut took = false;
    for digit in pressed {
        // A digit may have sent something, which puts the sheet away, and
        // every digit after it would then be read against a permanent that is
        // no longer being asked about.
        if duel.ability_menu.is_none() {
            break;
        }
        took |= sheet_digit(duel, digit);
    }
    took
}

/// One digit, against the sheet as it stands: a row, or the pager. Returns
/// whether it named either.
///
/// Split out of [`sheet_digits`] because that one takes a `MessageReader` and
/// this is the half worth testing — arming, sending and paging are the
/// two-stage mechanic, and a test that had to build keyboard events to reach
/// them would be testing Bevy.
///
/// The list is rebuilt here for every digit, because the one before it may
/// have sent something: a list read once and used twice is the chooser bug
/// this whole path was written to avoid.
pub fn sheet_digit(duel: &mut Duel, digit: char) -> bool {
    let Some(object) = duel.ability_menu else {
        return false;
    };
    let Some(options) = abilities_of(duel, object).filter(|o| o.len() > 1) else {
        return false;
    };
    let page = abilitysheet::clamp(options.len(), duel.ability_page);
    duel.ability_page = page;
    if digit == abilitysheet::PAGER {
        // The pager is drawn only where there is a second page, so a `0` on a
        // sheet of two rows is a key nothing on screen offered: it consumes
        // no frame and turns nothing.
        turn_the_page(duel);
        return abilitysheet::paged(options.len());
    }
    let Some(at) = abilitysheet::option_of(options.len(), page, digit) else {
        return false;
    };
    take_sheet_row(duel, at);
    true
}

/// The row a press landed on: armed by the first press, sent by the second.
///
/// The digit and the click share it, because they are the same press — the
/// roundel is what the digit is drawn on, and a row whose click armed while
/// its digit sent would be two controls wearing one number.
fn take_sheet_row(duel: &mut Duel, at: usize) {
    let Some(object) = duel.ability_menu else {
        return;
    };
    let Some(option) = abilities_of(duel, object).and_then(|o| o.get(at).cloned()) else {
        return;
    };
    duel.ability_pick = at;
    let armed = duel.armed.as_ref().is_some_and(|a| {
        a.object == object && matches!(&a.deed, Deed::Ability(action) if *action == option.action)
    });
    match abilitysheet::press(option.mana || option.tap_only, armed) {
        // Through `fire_armed` and not `duel.submit`, so the deed is
        // re-resolved against the current `LegalActions` exactly as the
        // confirm key and the second tap on the card already do.
        abilitysheet::Press::Fire => fire_armed(duel),
        abilitysheet::Press::Send | abilitysheet::Press::Arm => {
            if arm_ability(duel, object, &option) {
                duel.ability_menu = None;
            }
        }
    }
}

/// The tenth row: the next page of the ability sheet, wrapping at the end.
///
/// The cursor goes to the top of the new page rather than staying where it
/// was, because the page and the cursor are two ways of saying the same
/// thing — [`ability_menu_keys`] derives the page from the cursor when the
/// arrow keys walk off the end of one, and this is the same equality read the
/// other way round.
///
/// Wrapping rather than stopping: the pager is one key and there is no second
/// one for going back, so a player who overshoots gets there by pressing it
/// again.
fn turn_the_page(duel: &mut Duel) {
    let Some(object) = duel.ability_menu else {
        return;
    };
    let Some(options) = abilities_of(duel, object) else {
        return;
    };
    if !abilitysheet::paged(options.len()) {
        return;
    }
    let page = abilitysheet::clamp(options.len(), duel.ability_page);
    duel.ability_page = abilitysheet::turn(options.len(), page);
    duel.ability_pick = abilitysheet::rows(options.len(), duel.ability_page).start;
}

/// The card cursor, and the key that acts on what it is over. Returns whether
/// it consumed the frame.
fn move_the_cursor(fired: Fired, duel: &mut Duel) -> bool {
    for (action, (d_row, d_col)) in [
        (Action::CursorUp, (1, 0)),
        (Action::CursorDown, (-1, 0)),
        (Action::CursorLeft, (0, -1)),
        (Action::CursorRight, (0, 1)),
    ] {
        if fired.has(action) {
            move_cursor(duel, d_row, d_col);
        }
    }
    if fired.has(Action::ActivateCard)
        && let Some(object) = duel.hovered
    {
        activate_card(duel, object);
        return true;
    }
    false
}

/// Combat: where the next declaration points, and the answer that declares
/// nothing. Returns whether it consumed the frame.
fn aim_and_declare(fired: Fired, duel: &mut Duel) -> bool {
    let step = i32::from(fired.has(Action::CombatFocusNext))
        - i32::from(fired.has(Action::CombatFocusPrev));
    if step != 0 {
        cycle_combat_focus(duel, step);
    }
    if fired.has(Action::CombatNone) {
        declare_nothing(duel);
        return true;
    }
    false
}

/// The primary key, with its fixed precedence: the card under the cursor,
/// then the selected phase button, then confirm. Returns whether it consumed
/// the frame.
fn the_click(fired: Fired, duel: &mut Duel, prefs: &mut crate::prefs::Prefs) -> bool {
    if !fired.has(Action::Primary) {
        return false;
    }
    if let Some(object) = duel.hovered {
        activate_card(duel, object);
        return true;
    }
    if let Some((side, row)) = prefs.orders().selected() {
        prefs.edit().orders.toggle(side, row);
        return true;
    }
    if let Some(action) = duel.interaction.as_ref().and_then(Interaction::confirm) {
        duel.submit(action);
        return true;
    }
    false
}

/// Every straight answer to a pending choice.
///
/// Each goes through the interaction, which refuses it unless the engine
/// actually asked — so a key bound to "yes" does nothing at all during
/// combat, without this function knowing what combat is.
fn answer_the_question(fired: Fired, duel: &mut Duel, prefs: &mut crate::prefs::Prefs) {
    // Confirm / pass priority. Never toggles anything else, so it is the one
    // key that always means "I am done here".
    if fired.has(Action::Confirm)
        && let Some(action) = duel.interaction.as_ref().and_then(Interaction::confirm)
    {
        duel.submit(action);
        return;
    }
    for (action, answer) in [(Action::MulliganKeep, true), (Action::MulliganTake, false)] {
        if fired.has(action)
            && let Some(sent) = duel
                .interaction
                .as_ref()
                .and_then(|i| i.answer_mulligan(answer))
        {
            duel.submit(sent);
            return;
        }
    }
    for (action, answer) in [(Action::AnswerYes, true), (Action::AnswerNo, false)] {
        if fired.has(action)
            && let Some(sent) = duel
                .interaction
                .as_ref()
                .and_then(|i| i.answer_yes_no(answer))
        {
            duel.submit(sent);
            return;
        }
    }

    // Number choices step, and the interaction clamps the value to the
    // offered range — a player can hold a key without producing something the
    // engine would reject.
    let step = i32::from(fired.has(Action::NumberUp)) - i32::from(fired.has(Action::NumberDown));
    if step != 0 {
        step_number(duel, step);
    }

    // Cancel: an open preview first, then the zone browser, then a selected
    // phase button, then a half-built answer.
    //
    // The browser sits where it does because Esc walks the screen from the
    // top down and the sheet is a *standing* panel: the preview is over it
    // and is gone the moment the pointer moves, while the sheet stays until
    // it is put away. Its filter box comes earlier still, in `browser_keys` —
    // a box that has the keyboard answers Escape itself.
    if fired.has(Action::Cancel) {
        if duel.hovered.is_some() {
            duel.hovered = None;
            duel.hovered_at = None;
        } else if duel.browser.is_open() {
            duel.browser.close();
        } else if prefs.orders().selected().is_some() {
            prefs.rail_cursor().clear_selection();
        } else if let Some(i) = duel.interaction.as_mut() {
            i.cancel();
        }
    }
}

/// Moves a number choice by one, in whichever direction.
///
/// The one door for both arms of the stepper and both keys, so a click and a
/// key cannot come to disagree about what "up" is. The interaction clamps, so
/// holding a key stops at the boundary rather than producing something the
/// engine would reject.
fn step_number(duel: &mut Duel, delta: i32) {
    if let Some(i) = duel.interaction.as_mut() {
        let next = if delta > 0 {
            i.number().saturating_add(1)
        } else {
            i.number().saturating_sub(1)
        };
        i.set_number(next);
    }
}

/// Aims the next declaration at the next defender (or attacker).
fn cycle_combat_focus(duel: &mut Duel, delta: i32) {
    if let Some(i) = duel.interaction.as_mut() {
        i.cycle_focus(delta);
    }
}

/// Declares nothing and moves on — no attackers, or no blockers.
///
/// Routed through the interaction rather than sent as an empty action
/// directly, so an empty declaration is validated exactly like a full one and
/// the key does nothing at all outside combat.
fn declare_nothing(duel: &mut Duel) {
    let Some(i) = duel.interaction.as_mut() else {
        return;
    };
    if !i.is_combat() {
        return;
    }
    i.cancel();
    if let Some(action) = i.confirm() {
        duel.submit(action);
    }
}

/// Moves and stretches the zone browser's sheet.
///
/// # Why this is not `Pointer<Drag>`
///
/// Bevy's picking backend has a perfectly good drag gesture, and the lobby
/// uses it. The duel HUD cannot: it is a retained tree rebuilt from scratch
/// whenever [`crate::hud::HudRevision`] changes — a new snapshot, a hover, a
/// selection — so the header entity a drag chain was bound to is despawned
/// mid-gesture and the drag simply stops. A press that records *what* is
/// being held, and a per-frame read of the cursor, survive the rebuild
/// because neither of them holds an entity.
///
/// # Why the geometry is not in the revision
///
/// For the same reason from the other side: routing a drag through
/// `HudRevision` would rebuild two hundred nodes for every pixel of it. The
/// system writes the sheet's own `Node` and the in-memory settings, and the
/// next rebuild — whenever it happens, for whatever reason — reads the
/// settings and lands where the pointer left it. Only the *release* touches
/// the disk.
///
/// There is no easing here and that is deliberate: the house curve is for
/// things that move on their own, and a sheet that lagged the hand dragging
/// it would be wrong at every rate. `reduce_motion` therefore has nothing to
/// gate.
#[allow(clippy::too_many_arguments)] // two message readers, three queries, two stores
pub fn tray_drag(
    mut downs: MessageReader<Pointer<Press>>,
    mut ups: MessageReader<Pointer<Release>>,
    grips: Query<&crate::hud::TrayGrip>,
    corners: Query<&crate::hud::TrayResize>,
    closes: Query<&TrayClose>,
    parents: Query<&ChildOf>,
    windows: Query<&Window>,
    mut panels: Query<&mut Node, With<crate::hud::TrayPanel>>,
    mut duel: ResMut<Duel>,
    mut settings: ResMut<ClientSettings>,
) {
    use crate::hud::{TrayDrag, TrayDragKind};

    /// How far the pointer may wander and still have *clicked* the corner.
    ///
    /// A hand on a button moves a pixel or two between the press and the
    /// release; a resize that only moved four is a resize nobody meant.
    const TAP_SLOP: f32 = 4.0;

    // A sheet a *question* opened is not furniture the player arranged: it is
    // centred on whatever window it meets and reads no stored rectangle at all
    // (`Browser::placement`). Dragging it would write a rectangle nobody is
    // looking at into `settings.zone_browser` — the sheet would snap back to
    // the middle at the next rebuild, and the hand-opened sheet would later
    // stand where nobody put it. Only a `ByHand` sheet is furniture.
    if duel.browser.for_choice() {
        duel.tray_drag = None;
        return;
    }

    let cursor = windows.single().ok().and_then(Window::cursor_position);
    for down in downs.read() {
        // The ✕ sits *on* the header, so its lineage carries the grip. The
        // specific control claims the press before the row it stands on does,
        // or every close would first nudge the sheet by whatever the hand
        // wobbled between the press and the release — and then save it.
        if find_in_lineage(down.entity, &closes, &parents).is_some() {
            continue;
        }
        let kind = if find_in_lineage(down.entity, &corners, &parents).is_some() {
            Some(TrayDragKind::Resize)
        } else if find_in_lineage(down.entity, &grips, &parents).is_some() {
            Some(TrayDragKind::Move)
        } else {
            None
        };
        // A press with no cursor is a press from a harness that never moved
        // one; starting a drag from it would take the first real cursor
        // position as a delta and throw the sheet across the band.
        if let (Some(kind), Some(at)) = (kind, cursor) {
            duel.tray_drag = Some(TrayDrag {
                kind,
                origin: at,
                last: at,
            });
        }
    }
    let mut released = None;
    for _up in ups.read() {
        released = released.or_else(|| duel.tray_drag.take());
        duel.tray_drag = None;
    }

    if let (Some(drag), Some(at)) = (duel.tray_drag, cursor) {
        let band = crate::hud::band_of(&windows);
        let delta = at - drag.last;
        if delta != Vec2::ZERO {
            let place = settings
                .zone_browser
                .map_or_else(|| Placement::centred(band), |p| p.fit(band));
            let moved = match drag.kind {
                TrayDragKind::Move => place.moved_by((delta.x, delta.y), band),
                TrayDragKind::Resize => place.resized_by((delta.x, delta.y), band),
            };
            settings.zone_browser = Some(moved);
            write_placement(&mut panels, moved);
        }
        duel.tray_drag = Some(TrayDrag { last: at, ..drag });
    }

    // A press and a release on the corner with nothing between them is a
    // *click*, and the corner draws a ⤢ — so it is read as a maximise button
    // and has to be one. It could not be found by asking Bevy for a
    // `Pointer<Click>`: a resize ends over the corner too, because the corner
    // travels under the hand, so every drag would fire it.
    if let Some(drag) = released {
        if drag.kind == TrayDragKind::Resize && drag.last.distance(drag.origin) < TAP_SLOP {
            let band = crate::hud::band_of(&windows);
            let now = settings
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
            settings.zone_browser = Some(next);
            write_placement(&mut panels, next);
        }
        settings.save();
    }
}

/// Puts a placement on whatever sheet is currently drawn.
///
/// The renderer would get there on its own at the next rebuild — the sheet is
/// built from `settings.zone_browser` — but only at the next rebuild, and a
/// drag deliberately does not cause one. See [`tray_drag`]'s own docs for why
/// the geometry is kept out of the revision.
fn write_placement(panels: &mut Query<&mut Node, With<crate::hud::TrayPanel>>, place: Placement) {
    for mut node in panels {
        node.left = px(place.left);
        node.top = px(place.top);
        node.width = px(place.width);
        node.height = px(place.height);
    }
}

/// Drags on the preview's resize handle, and the resize shortcut
/// (Command/Alt + Shift + Up/Down). The size is persisted.
#[allow(clippy::too_many_arguments)] // events + queries + state, all needed
pub fn preview_resize(
    keys: Res<ButtonInput<KeyCode>>,
    mut downs: MessageReader<Pointer<Press>>,
    mut ups: MessageReader<Pointer<Release>>,
    resize: Query<&PreviewResize>,
    parents: Query<&ChildOf>,
    mut motions: MessageReader<MouseMotion>,
    mut duel: ResMut<Duel>,
    mut settings: ResMut<ClientSettings>,
) {
    for down in downs.read() {
        if find_in_lineage(down.entity, &resize, &parents).is_some() {
            duel.resize_drag = true;
        }
    }
    let mut ended = false;
    for _up in ups.read() {
        ended |= duel.resize_drag;
        duel.resize_drag = false;
    }
    if duel.resize_drag {
        let dx: f32 = motions.read().map(|m| m.delta.x).sum();
        if dx != 0.0 {
            settings.preview_scale = (settings.preview_scale + dx * 0.004).clamp(0.5, 1.75);
        }
    } else {
        motions.clear();
    }
    if ended {
        settings.save();
    }

    // Command/Alt + Shift + Up/Down resizes too.
    let meta = keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight)
        || keys.pressed(KeyCode::AltLeft)
        || keys.pressed(KeyCode::AltRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if meta && shift {
        if keys.just_pressed(KeyCode::ArrowUp) {
            settings.preview_scale = (settings.preview_scale + 0.05).clamp(0.5, 1.75);
            settings.save();
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            settings.preview_scale = (settings.preview_scale - 0.05).clamp(0.5, 1.75);
            settings.save();
        }
    }
}

/// Frames the next opponent's board, wrapping back to your own.
///
/// One key that walks the table rather than a numbered key per chair: a
/// four-seat game has three opponents, and a binding screen listing nine
/// "focus seat N" rows would be listing six that can never fire.
fn focus_next_opponent(duel: &mut Duel, rig: &mut crate::table::CameraRig) {
    let Some(board) = duel.board.as_ref() else {
        return;
    };
    let opponents: Vec<_> = board
        .pods
        .iter()
        .filter(|p| !p.is_local)
        .map(|p| p.player)
        .collect();
    if opponents.is_empty() {
        return;
    }
    let next = match duel.focus {
        None => Some(opponents[0]),
        Some(current) => opponents
            .iter()
            .position(|p| *p == current)
            .and_then(|i| opponents.get(i + 1).copied()),
    };
    match next {
        Some(player) => navigate_to_player(duel, rig, player),
        // Past the last opponent is home again, so the key never dead-ends.
        None => navigate_home(duel, rig),
    }
}

/// The selectable cards as a row grid: hand at the bottom, then each
/// seat's lanes from the local seat outward, then the stack. Row order
/// matches the visual layout, so W/S moves the way the eye expects.
///
/// The stack is last because it is drawn highest — the panel is pinned to the
/// top right — and it is here at all because a spell on the stack is a legal
/// target ("target spell", every counterspell in the game) and the keyboard
/// could not reach one. The grid was built out of the *board*, and the stack
/// is not on the board: the cursor walked the hand and the lanes and simply
/// never arrived, so a `ChooseTargets` naming only a spell was a question
/// with no keyboard answer. The pointer's half of the same gap is the row
/// carrying `HandCardVisual`; this is the other half.
fn cursor_grid(duel: &Duel) -> Vec<Vec<ObjectId>> {
    let Some(board) = duel.board.as_ref() else {
        return Vec::new();
    };
    let mut rows: Vec<Vec<ObjectId>> = Vec::new();
    let hand: Vec<ObjectId> = board.hand.iter().map(|c| c.id).collect();
    if !hand.is_empty() {
        rows.push(hand);
    }
    for pod in board
        .pods
        .iter()
        .filter(|p| p.is_local)
        .chain(board.pods.iter().filter(|p| !p.is_local))
    {
        for lane in &pod.lanes {
            let row: Vec<ObjectId> = lane.groups.iter().map(|g| g.representative).collect();
            if !row.is_empty() {
                rows.push(row);
            }
        }
    }
    // Top of the stack first, which is the order the panel draws and the
    // order the objects resolve in — so `A`/`D` walks the queue downwards.
    let stack: Vec<ObjectId> = board.stack.iter().map(|item| item.id).collect();
    if !stack.is_empty() {
        rows.push(stack);
    }
    rows
}

/// Moves the card cursor; wraps inside a row and clamps the column when
/// changing rows. With no cursor yet, starts at the first hand card.
/// Walks the keyboard cursor over the hand and the lanes.
///
/// Every arm here clears `hovered_at`, because the keyboard cursor is not on
/// the screen: it names a card, not a place. Left behind, the anchor would
/// still be wherever the pointer last stopped, and the preview would open
/// beside a card the cursor walked away from three presses ago.
fn move_cursor(duel: &mut Duel, d_row: i32, d_col: i32) {
    let grid = cursor_grid(duel);
    if grid.is_empty() {
        return;
    }
    let Some(current) = duel.hovered else {
        duel.hovered = Some(grid[0][0]);
        duel.hovered_at = None;
        return;
    };
    let Some((mut row, mut col)) = grid.iter().enumerate().find_map(|(r, row)| {
        row.iter()
            .position(|&id| id == current)
            .map(|c| (r as i32, c as i32))
    }) else {
        duel.hovered = Some(grid[0][0]);
        duel.hovered_at = None;
        return;
    };
    if d_col != 0 {
        let len = grid[row as usize].len() as i32;
        col = (col + d_col).rem_euclid(len);
    }
    if d_row != 0 {
        row = (row + d_row).rem_euclid(grid.len() as i32);
        col = col.min(grid[row as usize].len() as i32 - 1);
    }
    duel.hovered = Some(grid[row as usize][col as usize]);
    duel.hovered_at = None;
}

/// Sends the ability one row of the ability sheet stands for.
///
/// It is [`take_sheet_row`] and nothing else: a click on a row and the digit
/// drawn on that row are the same press, so the row arms on the first and
/// sends on the second exactly as the keyboard does — and the list is rebuilt
/// from `LegalActions` in there rather than trusted from the sheet, because a
/// sheet drawn a frame ago must not be able to send an ability the engine has
/// since stopped offering.
fn pick_ability(duel: &mut Duel, index: usize) {
    take_sheet_row(duel, index);
}

/// Answers an indexed choice: a colour, a seat, one of several ways to cast.
///
/// It answers on the click that picks it -- there is no second "OK", because
/// there is nothing to combine. The rows are rebuilt from the *current*
/// prompt first, so a button drawn before the engine moved on answers nothing
/// rather than the wrong thing.
fn pick_choice(duel: &mut Duel, index: usize) {
    let offered = duel
        .interaction
        .as_ref()
        .map(baylee_client_core::Interaction::prompt)
        // The language is irrelevant here and deliberately not plumbed: only
        // the *shape* of the answer is read back -- whether this prompt is an
        // indexed choice at all, and how many rows it has. The labels are the
        // renderer's business.
        .and_then(|p| {
            crate::choices::options(
                &p,
                baylee_client_core::Lang::En,
                duel.statics.as_ref(),
                &duel.subtype_filter,
            )
        })
        // Not `index < rows.len()`: a filtered list's rows carry the
        // engine's own indices, and most of them are not on screen.
        .is_some_and(|rows| rows.iter().any(|row| row.index == index));
    if !offered {
        return;
    }
    let action = duel
        .interaction
        .as_mut()
        .and_then(|i| i.choose_index(index).then(|| i.confirm())?);
    if let Some(action) = action {
        duel.submit(action);
    }
}

/// The two buttons in the top-right menu.
///
/// `was_armed` is the concession's state *before* this click, taken once at
/// the top of the loop: every click disarms, so the second press only counts
/// when nothing happened in between.
fn menu_click(duel: &mut Duel, action: MenuAction, was_armed: bool) {
    match action {
        // Two presses, because there is no undo behind this one.
        MenuAction::Concede => {
            if was_armed {
                duel.submit(PlayerAction::Concede);
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
            }
        }
        // Through the same door as the keys, and re-checked for the same
        // reason: the button is only drawn while a hold is running, and
        // `hold_action` reads the *current* view rather than the one that was
        // drawn — so a hold the engine has already expired cannot be
        // "cancelled" into a new one by a stale button.
        MenuAction::ReleaseHold => {
            if duel.priority_held()
                && let Some(action) = duel.hold_action(false)
            {
                duel.submit(action);
            }
        }
        // The same door the keys use, so the two ways of confirming cannot
        // drift; `fire_armed` re-resolves against the current `LegalActions`.
        MenuAction::SendArmed => fire_armed(duel),
        MenuAction::CancelArmed => duel.armed = None,
    }
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
fn browser_click(
    duel: &mut Duel,
    entity: Entity,
    tray: &TrayWidgets,
    parents: &Query<&ChildOf>,
) -> bool {
    if let Some(card) = find_in_lineage(entity, &tray.cards, parents) {
        activate_card(duel, card.object);
        return true;
    }
    // A tab inside the open tray switches zone; a chip outside it opens and
    // closes the whole panel. Two different jobs, so two components.
    if let Some(tab) = find_in_lineage(entity, &tray.tabs, parents) {
        duel.browser.show(tab.zone);
        return true;
    }
    if find_in_lineage(entity, &tray.close, parents).is_some() {
        duel.browser.close();
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
    // The filter box takes the keyboard on the click and gives it back on
    // the next one, so a player can leave the panel open and keep playing.
    if find_in_lineage(entity, &tray.filter, parents).is_some() {
        if duel.browser.is_typing() {
            duel.browser.stop_typing();
        } else {
            duel.browser.start_typing();
        }
        return true;
    }
    false
}

/// Pointer handling: clicking a card, a player tab, or a rail button.
///
/// A click means "this object", and what that does depends entirely on the
/// pending choice: it selects a target, declares an attacker, or plays a card.
/// Resolving that here rather than in the renderer keeps one place where a
/// click becomes an action.
#[allow(clippy::too_many_arguments)] // one query per clickable widget kind
pub fn pointer(
    mut clicks: MessageReader<Pointer<Click>>,
    cards: Query<&CardVisual>,
    hand_cards: Query<&HandCardVisual>,
    tabs: Query<&PlayerTab>,
    seat_steps: Query<&crate::hud::SeatStep>,
    menu_buttons: Query<&MenuButton>,
    prompt_buttons: Query<&PromptButton>,
    ability_buttons: Query<&AbilityButton>,
    pagers: Query<&crate::hud::SheetPager>,
    choice_buttons: Query<&ChoiceButton>,
    tray: TrayWidgets,
    parents: Query<&ChildOf>,
    mut duel: ResMut<Duel>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut rig: ResMut<crate::table::CameraRig>,
    mut touched: ResMut<crate::touch::Touched>,
) {
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
            let answer = activate_card(&mut duel, object);
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
                && i.toggle_player(tab.player) != SelectionOutcome::Rejected
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
        if let Some(button) = find_in_lineage(e, &ability_buttons, &parents) {
            pick_ability(&mut duel, button.index);
            continue;
        }
        if find_in_lineage(e, &pagers, &parents).is_some() {
            turn_the_page(&mut duel);
            continue;
        }
        if let Some(button) = find_in_lineage(e, &choice_buttons, &parents) {
            pick_choice(&mut duel, button.index);
            continue;
        }
        if browser_click(&mut duel, e, &tray, &parents) {
            continue;
        }
        if let Some(button) = find_in_lineage(e, &prompt_buttons, &parents) {
            let action = match button.action {
                PromptAction::Yes => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_yes_no(true)),
                PromptAction::No => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_yes_no(false)),
                PromptAction::Keep => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_mulligan(true)),
                PromptAction::Mulligan => duel
                    .interaction
                    .as_ref()
                    .and_then(|i| i.answer_mulligan(false)),
                PromptAction::Confirm => duel.interaction.as_ref().and_then(Interaction::confirm),
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
    mut last: Local<Option<ObjectId>>,
    cards: Query<&CardVisual>,
    hand_cards: Query<&HandCardVisual>,
    tray_cards: Query<&TrayCard>,
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
    }

    // A hovered card can leave without ever firing an `Out`, and playing the
    // card under the pointer is the ordinary way into that: the hand bar is
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
            HoverSource::Table => cards.iter().any(|v| v.object == object),
            // A tray row lives and dies with the panel: closing the browser,
            // switching its tab or typing into its filter rebuilds the whole
            // grid, and the row the pointer was over is gone without ever
            // firing an `Out`. Held against the tray's own rows for exactly
            // the reason the two above are held against theirs.
            HoverSource::Tray => tray_cards.iter().any(|t| t.object == object),
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
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum HoverSource {
    /// A hand card, or a card in the command zone.
    Hand,
    /// A permanent on the table.
    Table,
    /// A row in the zone browser.
    Tray,
    /// The keyboard cursor, or nothing at all.
    #[default]
    Elsewhere,
}

/// The battlefield camera: arrows pan, right- or middle-drag pans, the wheel
/// zooms unless the interface asked for it first, and two fingers do what two
/// fingers do (pan, pinch).
///
/// # The left button plays and never moves the camera
///
/// It used to orbit — a drag turned the table and tilted it — and the left
/// button is also the button that plays cards, so *every click that travelled
/// a pixel turned the table a little*. That is the owner's report
/// („kaum berühre ich mit der Maus was, drehe ich den Tisch etwas"), and the
/// half of it nobody could see was worse: [`crate::table::frame_table`] stops
/// following its own framing the moment the rig is not exactly the one it
/// computed, so a stray pixel switched the automatic framing off for the rest
/// of the session and the table stayed wherever the accident left it.
///
/// So orbit is gone, and with it the two controls that could only be set
/// wrong: **yaw** (every seat's bar is already drawn upright on its own mat,
/// so turning the table only makes "which side am I on" ambiguous) and
/// **lean** (`table::CAMERA_LEAN` is a measured trade between a card reading
/// as an object and a far seat's cards shrinking — a few degrees wide, and
/// nothing a hand aims). Shift+arrows and the touch rotation gesture went
/// with them.
///
/// What is left is the one job the camera has: **a table that does not fit
/// the window**. Zoom and pan, both recoverable with one key
/// ([`Action::FocusHome`]), and everything else is a *viewpoint* —
/// [`navigate_home`] and [`navigate_to_player`].
#[allow(clippy::too_many_arguments)]
pub fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut motions: MessageReader<bevy::input::mouse::MouseMotion>,
    mut wheels: MessageReader<MouseWheel>,
    mut scrolled: MessageReader<Pointer<bevy::picking::events::Scroll>>,
    nodes: Query<(), With<ComputedNode>>,
    mut pans: MessageReader<bevy::input::gestures::PanGesture>,
    mut pinches: MessageReader<bevy::input::gestures::PinchGesture>,
    mut duel: ResMut<Duel>,
    mut rig: ResMut<crate::table::CameraRig>,
) {
    // Screen-relative directions on the table plane.
    let right = Vec2::new(rig.yaw.cos(), -rig.yaw.sin());
    let forward = Vec2::new(-rig.yaw.sin(), -rig.yaw.cos());

    // The arrows are `NumberUp`/`NumberDown` in the standard keymap, and this
    // function reads `KeyCode` directly rather than through it — so while a
    // number is being chosen the same press was both raising X and panning the
    // table under it. The choice wins; the mouse and the touch gestures below
    // are untouched, because neither of them is bound to anything.
    let stepping = matches!(
        duel.interaction.as_ref().map(Interaction::prompt),
        Some(Prompt::ChooseNumber { .. })
    );

    // ---- keyboard: pan ---------------------------------------------------
    let pan_step = rig.distance * 0.02;
    if !stepping {
        if keys.pressed(KeyCode::ArrowLeft) {
            rig.target -= right * pan_step;
            duel.camera_held = true;
        }
        if keys.pressed(KeyCode::ArrowRight) {
            rig.target += right * pan_step;
            duel.camera_held = true;
        }
        if keys.pressed(KeyCode::ArrowUp) {
            rig.target += forward * pan_step;
            duel.camera_held = true;
        }
        if keys.pressed(KeyCode::ArrowDown) {
            rig.target -= forward * pan_step;
            duel.camera_held = true;
        }
    }

    // ---- mouse: right- or middle-drag pans -------------------------------
    //
    // Neither button plays a card, which is the whole reason they are the
    // ones that move the view: a gesture that moves the camera has to be a
    // gesture that can mean nothing else.
    let (mut dx, mut dy) = (0.0, 0.0);
    for motion in motions.read() {
        dx += motion.delta.x;
        dy += motion.delta.y;
    }
    let drag_scale = rig.distance / 600.0;
    if (buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle))
        && (dx != 0.0 || dy != 0.0)
    {
        rig.target -= right * dx * drag_scale;
        rig.target -= forward * dy * drag_scale;
        duel.camera_held = true;
    }

    // ---- wheel: zoom, unless the interface asked for it first -------------
    //
    // The hand bar used to be carved out of this by a rectangle — the bottom
    // of the window, the hand's own height plus twenty — and every other
    // panel was the camera's by construction, which is why a wheel over a
    // library zoomed the table. `hud::scrolls` owns that question now and
    // this reads its answer off the same messages rather than off a flag one
    // of the two would have to set before the other ran.
    let theirs = crate::hud::wheel_is_the_interfaces(&mut scrolled, &nodes);
    for wheel in wheels.read() {
        if theirs || wheel.y == 0.0 {
            continue;
        }
        rig.distance = (rig.distance * (1.0 - wheel.y * 0.08)).clamp(
            crate::table::CameraRig::MIN_DISTANCE,
            crate::table::CameraRig::MAX_DISTANCE,
        );
        duel.camera_held = true;
    }

    // ---- touch gestures ---------------------------------------------------
    //
    // Two fingers pan and pinch; the rotation gesture is gone with the
    // orbit it was the touch twin of.
    for pan in pans.read() {
        rig.target -= right * pan.0.x * drag_scale;
        rig.target -= forward * pan.0.y * drag_scale;
        duel.camera_held = true;
    }
    for pinch in pinches.read() {
        rig.distance = (rig.distance / (1.0 + pinch.0 * 0.5)).clamp(
            crate::table::CameraRig::MIN_DISTANCE,
            crate::table::CameraRig::MAX_DISTANCE,
        );
        duel.camera_held = true;
    }
}

/// Navigates the camera to a seat.s pod, framing it in the free canvas
/// area (clear of the tab strip and the hand bar), cards upright. Also
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

#[cfg(test)]
mod tests {
    use baylee_client_core::interaction::Interaction;
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_engine::choice::{LegalActions, Pending, PlayerAction};

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    #[test]
    fn confirming_a_priority_choice_passes() {
        let i = Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    can_pass: true,
                    lands: vec![obj(1)],
                    castable: vec![],
                    mana_abilities: vec![],
                    abilities: vec![],
                    suspendable: vec![],
                }),
            },
            PlayerId::new(0),
        );
        assert_eq!(i.confirm(), Some(PlayerAction::PassPriority));
        // And a click on the land plays it instead of passing.
        assert_eq!(
            i.play_card(obj(1)),
            Some(PlayerAction::PlayLand { card: obj(1) })
        );
    }

    #[test]
    fn a_click_on_something_the_engine_did_not_offer_does_nothing() {
        let mut i = Interaction::new(
            Pending::ChooseTargets {
                player: PlayerId::new(0),
                options: vec![obj(1)],
                player_options: vec![],
                min: 1,
                max: 1,
                reason: baylee_engine::choice::TargetPrompt::Targets,
            },
            PlayerId::new(0),
        );
        i.toggle(obj(99));
        assert!(i.selected().next().is_none());
        assert!(!i.can_confirm());
    }

    /// A counterspell has to be able to name the spell it counters.
    ///
    /// A live duel stopped dead on a `ChooseTargets { options: [200], min: 1 }`
    /// whose only option was a spell on the stack. The model had always
    /// allowed it — `Interaction::toggle` takes "a permanent, a card in a
    /// zone, or a spell on the stack" — and there was no way to *say* it: the
    /// stack panel's one pickable node was the 66-pixel picture inside the
    /// row, every other node carried `Pickable::IGNORE`, and a question with
    /// a minimum of one cannot be passed. The row carries `HandCardVisual`
    /// now, so this goes through the real `pointer` system and the real
    /// message rather than calling `toggle` by hand — which is the only way
    /// the test can fail if the wiring is undone again.
    #[test]
    fn a_click_on_a_stack_row_answers_the_question_it_was_asked() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::touch::Touched>()
            .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Click>>()
            .insert_resource(crate::Duel {
                interaction: Some(baylee_client_core::interaction::Interaction::new(
                    Pending::ChooseTargets {
                        player: PlayerId::new(0),
                        options: vec![obj(200)],
                        player_options: vec![],
                        min: 1,
                        max: 1,
                        reason: baylee_engine::choice::TargetPrompt::Targets,
                    },
                    PlayerId::new(0),
                )),
                ..default()
            })
            .add_systems(Update, super::pointer);

        // The row as `spawn_stack_entry` builds it: the object it draws, and
        // the marker that says this node is the row rather than a chip.
        let row = app
            .world_mut()
            .spawn((
                crate::hud::HandCardVisual { object: obj(200) },
                crate::hud::StackRowCard,
            ))
            .id();
        click(&mut app, row);

        let duel = app.world().resource::<crate::Duel>();
        let picked: Vec<_> = duel
            .interaction
            .as_ref()
            .expect("the question is still standing")
            .selected()
            .collect();
        assert_eq!(picked, vec![obj(200)], "the spell on the stack was chosen");
        assert!(
            duel.interaction
                .as_ref()
                .is_some_and(baylee_client_core::Interaction::can_confirm),
            "and the answer is complete"
        );
    }

    /// The keyboard's half of the same gap.
    ///
    /// `cursor_grid` was built out of the *board* — the hand and each seat's
    /// lanes — and the stack is not on the board, so the card cursor walked
    /// past a spell it was being asked to target and never arrived. The stack
    /// is the last row because the panel is drawn highest.
    #[test]
    fn the_card_cursor_reaches_a_spell_on_the_stack() {
        use baylee_client_core::BoardModel;
        use baylee_client_core::board::Openings;
        use baylee_client_core::test_support::{ViewBuilder, printed};

        let view = ViewBuilder::new(2)
            .with_hand(vec![("Ornithopter", 0, 30)])
            .with_stack(vec![printed(200, 0, "Spellseeker", 4)])
            .build();
        let board = BoardModel::from_view(
            &view,
            Openings::none(),
            |_| 12.0,
            crate::cardart::registry(),
        );
        assert!(
            board.stack.iter().any(|item| item.id == obj(200)),
            "the board model carries the stack"
        );

        let mut duel = crate::Duel {
            board: Some(board),
            ..Default::default()
        };
        let grid = super::cursor_grid(&duel);
        assert_eq!(
            grid.last().map(Vec::as_slice),
            Some(&[obj(200)][..]),
            "the stack is the topmost row of the grid"
        );

        // From the hand, one step up the grid is the stack, because there is
        // nothing on either board between them.
        duel.hovered = Some(obj(30));
        super::move_cursor(&mut duel, 1, 0);
        assert_eq!(
            duel.hovered,
            Some(obj(200)),
            "the cursor walks onto the stack"
        );
    }

    /// The whole keyboard path, and not just the decision underneath it.
    ///
    /// `confirming_a_priority_choice_passes` asks the `Interaction` directly,
    /// which a live game showed is not enough: a land the engine had offered,
    /// sitting under the cursor, was played by no key and no click, and every
    /// unit test kept passing. So this one presses a `KeyCode` at the real
    /// system and reads the outbox — nothing hand-built in between.
    #[test]
    fn the_primary_key_plays_the_land_under_the_cursor() {
        use baylee_client_core::prefs::Action;
        use bevy::input::ButtonInput;
        use bevy::input::keyboard::KeyboardInput;
        use bevy::prelude::*;

        let duel = crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::Priority {
                    player: PlayerId::new(0),
                    legal: Box::new(LegalActions {
                        can_pass: true,
                        lands: vec![obj(3)],
                        castable: vec![],
                        mana_abilities: vec![],
                        abilities: vec![],
                        suspendable: vec![],
                    }),
                },
                PlayerId::new(0),
            )),
            hovered: Some(obj(3)),
            ..Default::default()
        };

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_message::<KeyboardInput>()
            .insert_resource(duel)
            .add_systems(Update, super::keyboard);

        // A precondition, so that a keymap this test cannot see is never the
        // reason it passes: it would pass by pressing nothing at all.
        {
            let prefs = app.world().resource::<crate::prefs::Prefs>();
            let mut probe = ButtonInput::<KeyCode>::default();
            probe.press(KeyCode::Enter);
            assert!(
                crate::keys::Fired::of(&probe, prefs.keymap()).has(Action::Primary),
                "enter is the primary key in the standard map"
            );
        }

        // Once. Playing a land is the one-click case: its whole cost is the
        // land drop and the worst it can go wrong is the wrong land in it.
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::Enter);
        }
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [PlayerAction::PlayLand { card: obj(3) }],
            "the land under the cursor is what the primary key plays"
        );
    }

    /// Presses a key at the real system and answers the outbox.
    ///
    /// Shares [`the_primary_key_plays_the_land_under_the_cursor`]'s shape
    /// rather than its body: the point of both is that nothing is hand-built
    /// between a `KeyCode` and a `PlayerAction`.
    fn pressing(pending: Pending, key: bevy::prelude::KeyCode) -> Vec<PlayerAction> {
        use bevy::input::ButtonInput;
        use bevy::input::keyboard::KeyboardInput;
        use bevy::prelude::*;

        let duel = crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                pending,
                PlayerId::new(0),
            )),
            ..Default::default()
        };

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_message::<KeyboardInput>()
            .insert_resource(duel)
            .add_systems(Update, super::keyboard);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world().resource::<crate::Duel>().outbox().to_vec()
    }

    /// Every straight answer to a question, through the keyboard.
    ///
    /// Written to settle entry 36 of `docs/observed-faults.md`, which says a
    /// yes/no question is answerable only with the pointer. Half of that is
    /// wrong on its face: `Action::AnswerYes` and `AnswerNo` exist,
    /// `Keymap::standard` binds them to `Y` and `N`, and
    /// `answer_the_question` reads both — so a live run in which `Y` did
    /// nothing was stopped by something else, and an entry naming the wrong
    /// mechanism sends the fix to the wrong file.
    ///
    /// The mulligan pair is here for the same reason: the entry names it as
    /// the same branch, and a claim about two branches wants both pressed.
    #[test]
    fn a_question_is_answered_from_the_keyboard() {
        use baylee_engine::choice::YesNoPrompt;
        use bevy::prelude::KeyCode;

        let yes_no = |prompt| Pending::YesNo {
            player: PlayerId::new(0),
            prompt,
            source: None,
        };
        assert_eq!(
            pressing(yes_no(YesNoPrompt::Generic), KeyCode::KeyY),
            [PlayerAction::YesNo(true)],
            "Y answers a generic yes/no"
        );
        assert_eq!(
            pressing(yes_no(YesNoPrompt::Generic), KeyCode::KeyN),
            [PlayerAction::YesNo(false)],
            "N answers a generic yes/no"
        );
        // The prompt the live run actually stalled on, in case the shape of
        // the question ever starts deciding whether it can be answered.
        assert_eq!(
            pressing(
                yes_no(YesNoPrompt::PayLifeOrEnterTapped { amount: 2 }),
                KeyCode::KeyY
            ),
            [PlayerAction::YesNo(true)],
            "Y pays the shockland"
        );

        let mulligan = Pending::Mulligan {
            player: PlayerId::new(0),
            taken: 0,
            next_is_free: true,
        };
        assert_eq!(
            pressing(mulligan.clone(), KeyCode::KeyK),
            [PlayerAction::MulliganKeep],
            "K keeps the hand"
        );
        assert_eq!(
            pressing(mulligan, KeyCode::KeyB),
            [PlayerAction::MulliganTake],
            "B takes a mulligan"
        );
    }

    /// The finger, the hand row and the real `pointer`, with a land to play.
    ///
    /// Everything a tap travels through except the tree that gets rebuilt
    /// underneath it — which is the point: the rebuild is done by hand in the
    /// tests below, because that is what the client does to itself on every
    /// hover change and every arriving view.
    fn hand_app() -> bevy::app::App {
        use bevy::picking::events::{Click, Pointer, Press, Release};
        use bevy::prelude::*;

        // Spelled out, because `bevy::prelude` brings a `bevy_ui::Interaction`
        // of its own and shadows the one this client means.
        let duel = crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::Priority {
                    player: PlayerId::new(0),
                    legal: Box::new(LegalActions {
                        can_pass: true,
                        lands: vec![obj(3)],
                        castable: vec![],
                        mana_abilities: vec![],
                        abilities: vec![],
                        suspendable: vec![],
                    }),
                },
                PlayerId::new(0),
            )),
            ..Default::default()
        };

        let mut app = App::new();
        app.init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::touch::Touched>()
            .add_message::<Pointer<Press>>()
            .add_message::<Pointer<Release>>()
            .add_message::<Pointer<Click>>()
            .insert_resource(duel)
            .add_systems(
                Update,
                (crate::touch::watch_the_finger, super::pointer).chain(),
            );
        let mut window = Window::default();
        window.resolution.set(1728.0, 1052.0);
        app.world_mut().spawn((window, bevy::window::PrimaryWindow));
        app
    }

    /// One node in the hand row, as `spawn_hand_bar` builds one.
    ///
    /// Both components, because the pair is what the two systems ask for:
    /// the wider one is what a press and a click resolve through, the marker
    /// is what says this node is the row's and not the stack panel's.
    fn row_card(app: &mut bevy::app::App, object: ObjectId) -> bevy::prelude::Entity {
        app.world_mut()
            .spawn((
                crate::hud::HandCardVisual { object },
                crate::hud::HandRowCard,
            ))
            .id()
    }

    /// Where the pointer is, as the picking backend would report it.
    fn pointer_at(app: &mut bevy::app::App) -> bevy::picking::pointer::Location {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::prelude::*;
        use bevy::window::{PrimaryWindow, WindowRef};

        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .expect("the harness made a window");
        let target = WindowRef::Entity(window)
            .normalize(Some(window))
            .expect("a window is a render target");
        bevy::picking::pointer::Location {
            target: NormalizedRenderTarget::Window(target),
            position: Vec2::ZERO,
        }
    }

    fn finger_down(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
        use bevy::picking::events::{Pointer, Press};
        use bevy::picking::pointer::PointerId;

        let location = pointer_at(app);
        let event = Press {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
            count: 1,
        };
        app.world_mut()
            .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
    }

    fn finger_up(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
        use bevy::picking::events::{Pointer, Release};
        use bevy::picking::pointer::PointerId;

        let location = pointer_at(app);
        let event = Release {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
        };
        app.world_mut()
            .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
    }

    /// The click bevy raises when the press and the release agree on an
    /// entity — the half that goes missing when the tree is rebuilt.
    fn the_click(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
        use bevy::picking::events::{Click, Pointer};
        use bevy::picking::pointer::PointerId;

        let location = pointer_at(app);
        let event = Click {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
            duration: std::time::Duration::from_millis(10),
            count: 1,
        };
        app.world_mut()
            .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
    }

    /// A tap the tree ate still plays the card.
    ///
    /// `docs/observed-faults.md` 35. Bevy raises a `Pointer<Click>` only when
    /// the press and the release land on the same **entity**, and the hand
    /// row is rebuilt on every hover change and on every arriving view — so
    /// a finger that is down across one of those comes up on a node born
    /// after the press and no click is ever raised. The card sank, came back
    /// and played nothing.
    #[test]
    fn a_tap_that_spans_a_rebuild_still_plays_the_card() {
        let mut app = hand_app();
        let before = row_card(&mut app, obj(3));
        finger_down(&mut app, before);
        app.update();

        // The rebuild: the node the finger went down on is despawned and the
        // same card comes back as a different entity.
        app.world_mut().entity_mut(before).despawn();
        let after = row_card(&mut app, obj(3));
        finger_up(&mut app, after);
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [PlayerAction::PlayLand { card: obj(3) }],
            "the tap reached the card it was made on"
        );
    }

    /// The other half of the fault text, and no rebuild in it at all.
    ///
    /// A card's art, its text and its rail are separate pickable children, so
    /// a press that drifts across that seam is two entities and bevy raises
    /// no click either. It rides on the same lineage walk a click does, which
    /// is why one fix covers both.
    #[test]
    fn a_press_that_drifts_across_one_card_is_still_a_tap() {
        use bevy::prelude::*;

        let mut app = hand_app();
        let card = row_card(&mut app, obj(3));
        let art = app.world_mut().spawn(ChildOf(card)).id();
        let text = app.world_mut().spawn(ChildOf(card)).id();

        finger_down(&mut app, art);
        finger_up(&mut app, text);
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [PlayerAction::PlayLand { card: obj(3) }],
            "the art and the text are one card"
        );
    }

    /// A tap the tree *did* hear is sent once, not twice.
    ///
    /// The counter-test the fix above is worth nothing without: the flag is
    /// raised by every release over the card the finger is on, including the
    /// ordinary ones, and is meant to be taken down again by the click that
    /// answers them. Read before the clicks instead of after, this plays the
    /// land and then plays it again.
    #[test]
    fn a_tap_the_tree_heard_is_sent_once() {
        let mut app = hand_app();
        let card = row_card(&mut app, obj(3));
        finger_down(&mut app, card);
        app.update();

        // Press and release on the same entity, so bevy raises the click too
        // — all three on the frame the release lands, as they arrive live.
        finger_up(&mut app, card);
        the_click(&mut app, card);
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [PlayerAction::PlayLand { card: obj(3) }],
            "one tap is one land"
        );
    }

    /// A press dragged off its card and let go over another one asks nothing.
    ///
    /// The second counter-test: a flag raised on every release, rather than
    /// only on a release over the card the finger went down on, would turn
    /// every dragged-off press into a tap on whatever it started on.
    #[test]
    fn a_press_let_go_over_another_card_sends_nothing() {
        let mut app = hand_app();
        let land = row_card(&mut app, obj(3));
        let other = row_card(&mut app, obj(5));
        finger_down(&mut app, land);
        app.update();
        finger_up(&mut app, other);
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [],
            "a press that moved on is not a tap"
        );
    }

    /// Builds the app the two menu-button tests share: the real `pointer`
    /// system, and a click helper that goes through the real message.
    fn menu_app(
        duel: crate::Duel,
    ) -> (bevy::app::App, bevy::prelude::Entity, bevy::prelude::Entity) {
        use crate::hud::MenuAction;
        use bevy::prelude::*;

        let mut app = App::new();
        app.init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::touch::Touched>()
            .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Click>>()
            .insert_resource(duel)
            .add_systems(Update, super::pointer);
        let draw = app
            .world_mut()
            .spawn(crate::hud::MenuButton {
                action: MenuAction::OfferDraw,
            })
            .id();
        let concede = app
            .world_mut()
            .spawn(crate::hud::MenuButton {
                action: MenuAction::Concede,
            })
            .id();
        (app, draw, concede)
    }

    /// One click on one entity, as the picking backend would report it.
    fn click(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::picking::events::{Click, Pointer};
        use bevy::picking::pointer::{Location, PointerId};
        use bevy::prelude::*;
        use bevy::window::{PrimaryWindow, WindowRef};

        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap_or_else(|_| {
                app.world_mut()
                    .spawn((Window::default(), PrimaryWindow))
                    .id()
            });
        let camera = app.world_mut().spawn_empty().id();
        let target = WindowRef::Entity(window)
            .normalize(Some(window))
            .expect("a window is a render target");
        let location = Location {
            target: NormalizedRenderTarget::Window(target),
            position: Vec2::ZERO,
        };
        let event = Click {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
            duration: std::time::Duration::from_millis(10),
            count: 1,
        };
        app.world_mut()
            .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
        app.update();
    }

    /// Playing the card under the pointer must take its preview with it.
    ///
    /// The hand bar is rebuilt whole on every board change, so the node the
    /// pointer was over is *despawned* — and Bevy fires no `Out` for an
    /// entity that no longer exists. The card preview therefore stayed open
    /// over the middle of the table until the player happened to hover
    /// something else, which is how a screenshot of a live game found it.
    ///
    /// The second half is the part a bare "does this object still exist"
    /// check would fail: a land goes on playing under the same `ObjectId`,
    /// now as a permanent, so the hover is only stale because it came from
    /// the *hand*.
    #[test]
    fn a_hand_card_that_is_played_takes_its_hover_with_it() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
            .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
            .add_message::<bevy::window::CursorMoved>()
            .insert_resource(crate::Duel::default())
            .add_systems(Update, super::pointer_hover);

        let card = app
            .world_mut()
            .spawn(crate::hud::HandCardVisual { object: obj(3) })
            .id();
        hover(&mut app, card);
        assert_eq!(
            app.world().resource::<crate::Duel>().hovered,
            Some(obj(3)),
            "the pointer over a hand card is a hover"
        );

        // The land is played: the hand bar is rebuilt without it, and the
        // same object arrives on the table. No `Out` is fired, and the
        // pointer does not move.
        app.world_mut().entity_mut(card).despawn();
        app.world_mut().spawn(crate::table::CardVisual {
            object: obj(3),
            count: 1,
        });
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().hovered,
            None,
            "a hand card that left the hand is not still hovered"
        );
    }

    /// The cure must not bring back the disease.
    ///
    /// `Duel::hovered` has four writers and `pointer_hover` is only one of
    /// them: the keyboard cursor writes it too, and the cursor walking off a
    /// permanent and onto a hand card is exactly that. Held against the
    /// *pointer's* last source, that write would be checked against the table,
    /// not found there and cleared on the next frame — the stall of "the
    /// pointer only speaks when it moves" back through a different door. So a
    /// hover this system did not write is nobody's kind in particular.
    #[test]
    fn a_hover_this_system_did_not_write_is_left_alone() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
            .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
            .add_message::<bevy::window::CursorMoved>()
            .insert_resource(crate::Duel::default())
            .add_systems(Update, super::pointer_hover);

        let permanent = app
            .world_mut()
            .spawn(crate::table::CardVisual {
                object: obj(7),
                count: 1,
            })
            .id();
        app.world_mut()
            .spawn(crate::hud::HandCardVisual { object: obj(3) });
        hover(&mut app, permanent);
        assert_eq!(app.world().resource::<crate::Duel>().hovered, Some(obj(7)));

        // What `move_cursor` does when the keyboard walks onto the hand: it
        // writes the hover directly, and the pointer has not moved.
        app.world_mut().resource_mut::<crate::Duel>().hovered = Some(obj(3));
        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().hovered,
            Some(obj(3)),
            "the keyboard cursor survives a pointer that is resting elsewhere"
        );
    }

    /// The other half of `a_hand_card_that_is_played_takes_its_hover_with_it`.
    ///
    /// The same event — the card under the hover is played, its hand node is
    /// despawned and a permanent appears under the same `ObjectId` — and the
    /// answer is the opposite one, because the two hovers are valid for
    /// different reasons. The pointer's is over an entity that no longer
    /// exists, so it goes. The keyboard's is a position in `cursor_grid`,
    /// the card is still in that grid one row down, and taking it away would
    /// send the player's next arrow key back to the start of their hand.
    ///
    /// Written as a test rather than left to the comment because the union in
    /// the `Elsewhere` arm reads like the permissive fallback of the other
    /// two, and the next person to tighten it will have this fail.
    #[test]
    fn the_keyboard_cursor_follows_a_card_it_played_onto_the_table() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
            .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
            .add_message::<bevy::window::CursorMoved>()
            .insert_resource(crate::Duel::default())
            .add_systems(Update, super::pointer_hover);

        // A land in hand, with the keyboard cursor on it: written straight to
        // the resource, which is what `move_cursor` does.
        let in_hand = app
            .world_mut()
            .spawn(crate::hud::HandCardVisual { object: obj(5) })
            .id();
        app.world_mut().resource_mut::<crate::Duel>().hovered = Some(obj(5));
        app.update();

        // It is played. The hand bar is rebuilt without it and the same
        // object is now a permanent.
        app.world_mut().entity_mut(in_hand).despawn();
        app.world_mut().spawn(crate::table::CardVisual {
            object: obj(5),
            count: 1,
        });
        app.update();

        assert_eq!(
            app.world().resource::<crate::Duel>().hovered,
            Some(obj(5)),
            "the cursor should follow the card it just played, not reset"
        );
    }

    /// One pointer entering one entity, as the picking backend would report
    /// it. The cursor move is what opens the system's grace window: a still
    /// pointer is deliberately silent.
    fn hover(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::picking::events::{Over, Pointer};
        use bevy::picking::pointer::{Location, PointerId};
        use bevy::prelude::*;
        use bevy::window::{PrimaryWindow, WindowRef};

        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap_or_else(|_| {
                app.world_mut()
                    .spawn((Window::default(), PrimaryWindow))
                    .id()
            });
        let camera = app.world_mut().spawn_empty().id();
        let target = WindowRef::Entity(window)
            .normalize(Some(window))
            .expect("a window is a render target");
        let location = Location {
            target: NormalizedRenderTarget::Window(target),
            position: Vec2::ZERO,
        };
        app.world_mut().write_message(bevy::window::CursorMoved {
            window,
            position: Vec2::ZERO,
            delta: None,
        });
        app.world_mut().write_message(Pointer::new(
            PointerId::Mouse,
            location,
            Over {
                hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
            },
            entity,
        ));
        app.update();
    }

    /// The engine refuses a draw offer outside the offerer's own priority, so
    /// the button used to be a live button whose usual answer was an error.
    #[test]
    fn a_draw_is_only_offered_from_this_seats_own_priority() {
        use bevy::prelude::*;

        // A choice that is not priority: the offer is not sent.
        let (mut app, draw, _) = menu_app(crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::YesNo {
                    player: PlayerId::new(0),
                    prompt: baylee_engine::choice::YesNoPrompt::Generic,
                    source: None,
                },
                PlayerId::new(0),
            )),
            ..Default::default()
        });
        click(&mut app, draw);
        assert!(
            app.world().resource::<crate::Duel>().outbox().is_empty(),
            "a draw was offered without priority, which the engine refuses"
        );

        // And with priority it goes.
        let (mut app, draw, _) = menu_app(crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::Priority {
                    player: PlayerId::new(0),
                    legal: Box::new(LegalActions {
                        can_pass: true,
                        lands: vec![],
                        castable: vec![],
                        mana_abilities: vec![],
                        abilities: vec![],
                        suspendable: vec![],
                    }),
                },
                PlayerId::new(0),
            )),
            ..Default::default()
        });
        click(&mut app, draw);
        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [PlayerAction::OfferDraw]
        );
    }

    /// One misclick used to end a ranked game.
    #[test]
    fn conceding_takes_two_presses_and_anything_else_forgets_the_first() {
        let (mut app, draw, concede) = menu_app(crate::Duel::default());

        click(&mut app, concede);
        assert!(
            app.world().resource::<crate::Duel>().outbox().is_empty(),
            "one press conceded the game"
        );
        assert!(app.world().resource::<crate::Duel>().concede_armed);

        // Anything else in between and the first press is forgotten.
        click(&mut app, draw);
        assert!(!app.world().resource::<crate::Duel>().concede_armed);
        click(&mut app, concede);
        assert!(app.world().resource::<crate::Duel>().outbox().is_empty());

        // Twice in a row, and it goes.
        click(&mut app, concede);
        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [PlayerAction::Concede]
        );
    }

    /// A number choice with a range, ready to be typed at.
    fn number_duel(max: u32) -> crate::Duel {
        crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::ChooseNumber {
                    player: PlayerId::new(0),
                    min: 0,
                    max,
                },
                PlayerId::new(0),
            )),
            ..Default::default()
        }
    }

    /// Stepping from 0 to 12 is twelve presses, and X is routinely somebody's
    /// whole hand of lands.
    #[test]
    fn a_number_can_be_typed_rather_than_stepped_to() {
        use bevy::input::ButtonInput;
        use bevy::input::keyboard::{Key, KeyboardInput};
        use bevy::prelude::*;

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_message::<KeyboardInput>()
            .insert_resource(number_duel(20))
            .add_systems(Update, super::keyboard);
        let window = app.world_mut().spawn_empty().id();

        let type_digit = |app: &mut App, c: char| {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::Digit0,
                logical_key: Key::Character(c.to_string().into()),
                state: bevy::input::ButtonState::Pressed,
                text: Some(c.to_string().into()),
                repeat: false,
                window,
            });
            app.update();
        };

        type_digit(&mut app, '1');
        type_digit(&mut app, '2');
        let number = |app: &App| {
            app.world()
                .resource::<crate::Duel>()
                .interaction
                .as_ref()
                .expect("the choice stands")
                .number()
        };
        assert_eq!(number(&app), 12, "a second digit appends");

        // …and a digit that would leave the range is the whole answer instead,
        // which is what a player means by typing 7 at a maximum of 20.
        type_digit(&mut app, '7');
        assert_eq!(number(&app), 7);

        // Backspace takes one off.
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Backspace,
            logical_key: Key::Backspace,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
        assert_eq!(number(&app), 0);
    }

    /// The graveyard is searchable, and only while it has been asked to be.
    ///
    /// "Sortierbar, durchsuchbar, scrollbar" — the first and the last were
    /// there and the middle one was not: `Browser::set_filter` was written
    /// and no key or click ever reached it. What is pinned here is both
    /// halves of the bargain, because the panel can stand open for a whole
    /// turn: letters reach the box once it holds the keyboard, and they must
    /// not before.
    #[test]
    fn the_pile_is_searchable_only_while_the_box_holds_the_keyboard() {
        use bevy::input::ButtonInput;
        use bevy::input::keyboard::{Key, KeyboardInput};
        use bevy::prelude::*;

        let mut app = App::new();
        let mut duel = crate::Duel::default();
        duel.browser.open();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_message::<KeyboardInput>()
            .insert_resource(duel)
            .add_systems(Update, super::keyboard);
        let window = app.world_mut().spawn_empty().id();

        // A letter is two things at once — a character for a text box and a
        // bound action for the game — and a real press sends both, so the
        // helper does too. Nothing here clears `ButtonInput` between frames
        // (that is `bevy_input`'s own system, and this `App` has none), so a
        // press has to be released by hand or every later frame sees it held.
        let type_letter = |app: &mut App, code: KeyCode, c: char| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(code);
            app.world_mut().write_message(KeyboardInput {
                key_code: code,
                logical_key: Key::Character(c.to_string().into()),
                state: bevy::input::ButtonState::Pressed,
                text: Some(c.to_string().into()),
                repeat: false,
                window,
            });
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
        };
        let open = |app: &App| app.world().resource::<crate::Duel>().browser.is_open();
        let filter = |app: &App| {
            app.world()
                .resource::<crate::Duel>()
                .browser
                .filter()
                .to_string()
        };

        // An open panel is not a focused box: the letters belong to the game.
        // `G` is `ToggleBrowser`, so the claim is not merely that the filter
        // stayed empty — a key that went nowhere at all would satisfy that,
        // and a panel that swallowed the keyboard for as long as it stood
        // open is the bug this bargain exists to prevent. The action has to
        // have *fired*.
        let was = open(&app);
        type_letter(&mut app, KeyCode::KeyG, 'g');
        assert_eq!(filter(&app), "", "the box typed without being asked to");
        assert_ne!(open(&app), was, "the open panel ate a bound key");

        app.world_mut().resource_mut::<crate::Duel>().browser.open();
        app.world_mut()
            .resource_mut::<crate::Duel>()
            .browser
            .start_typing();
        // And now the other way round: the same key is a letter, and the
        // panel must not close under the player's typing.
        let was = open(&app);
        type_letter(&mut app, KeyCode::KeyG, 'g');
        assert_eq!(filter(&app), "g");
        assert_eq!(open(&app), was, "typing a letter reached the game");

        app.world_mut()
            .resource_mut::<crate::Duel>()
            .browser
            .clear_filter();
        type_letter(&mut app, KeyCode::KeyM, 'm');
        type_letter(&mut app, KeyCode::KeyO, 'o');
        assert_eq!(filter(&app), "mo");

        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Backspace,
            logical_key: Key::Backspace,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
        assert_eq!(filter(&app), "m", "backspace did not reach the box");
    }

    /// The arrows are bound to `NumberUp`/`NumberDown`, and `camera_controls`
    /// reads `KeyCode` directly rather than through the keymap — so the same
    /// press was raising X and panning the table out from under it.
    #[test]
    fn the_arrows_do_not_pan_the_table_while_a_number_is_being_chosen() {
        use bevy::input::ButtonInput;
        use bevy::input::mouse::{MouseMotion, MouseWheel};
        use bevy::prelude::*;

        let run = |duel: crate::Duel| {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<ButtonInput<MouseButton>>()
                .init_resource::<crate::table::CameraRig>()
                .add_message::<MouseMotion>()
                .add_message::<MouseWheel>()
                .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Scroll>>()
                .add_message::<bevy::input::gestures::PanGesture>()
                .add_message::<bevy::input::gestures::PinchGesture>()
                .insert_resource(duel)
                .add_systems(Update, super::camera_controls);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ArrowUp);
            app.update();
            app.world().resource::<crate::table::CameraRig>().target
        };

        assert_eq!(
            run(number_duel(20)),
            crate::table::CameraRig::default().target,
            "the arrow panned the table while it was choosing a number"
        );
        assert_ne!(
            run(crate::Duel::default()),
            crate::table::CameraRig::default().target,
            "and with no number pending the arrow still pans"
        );
    }

    /// Two buttons move the table, and neither of them plays a card.
    ///
    /// This replaces a test that asserted the orbit convention — left-drag
    /// turns the table, right-drag slides it — which is the defect the owner
    /// reported: the left button is also the button that *plays*, so every
    /// click that travelled a pixel turned the table a little. Nothing turns
    /// it any more, so what is left to check is that a pan is still a pan,
    /// that it changes nothing else, and that it says the player is aiming.
    ///
    /// `table::framing_tests` holds the other half, which needs
    /// [`crate::table::frame_table`] running beside this one: a left drag
    /// moves nothing *and* leaves the automatic framing on.
    #[test]
    fn only_the_right_and_middle_buttons_move_the_table() {
        use bevy::input::ButtonInput;
        use bevy::input::mouse::{MouseMotion, MouseWheel};
        use bevy::picking::events::{Pointer, Scroll};
        use bevy::prelude::*;

        let drag = |button: MouseButton, delta: Vec2| {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<ButtonInput<MouseButton>>()
                .init_resource::<crate::table::CameraRig>()
                .add_message::<MouseMotion>()
                .add_message::<MouseWheel>()
                .add_message::<Pointer<Scroll>>()
                .add_message::<bevy::input::gestures::PanGesture>()
                .add_message::<bevy::input::gestures::PinchGesture>()
                .init_resource::<crate::Duel>()
                .add_systems(Update, super::camera_controls);
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(button);
            app.world_mut().write_message(MouseMotion { delta });
            app.update();
            (
                *app.world().resource::<crate::table::CameraRig>(),
                app.world().resource::<crate::Duel>().camera_held,
            )
        };

        let home = crate::table::CameraRig::default();
        for button in [MouseButton::Right, MouseButton::Middle] {
            let (moved, held) = drag(button, Vec2::new(40.0, 20.0));
            assert_ne!(
                moved.target, home.target,
                "{button:?} did not move the table"
            );
            assert!(
                (moved.yaw - home.yaw).abs() < 1e-6 && (moved.lean - home.lean).abs() < 1e-6,
                "{button:?} turned the table as well"
            );
            assert!(held, "{button:?} is the player aiming, and has to say so");
        }

        let (untouched, held) = drag(MouseButton::Left, Vec2::new(40.0, 0.0));
        assert_eq!(
            untouched, home,
            "the left button plays cards and nothing else"
        );
        assert!(!held, "and it does not take the camera off the table");
    }

    /// The browser had a pointer route and no keyboard one, which is exactly
    /// the promise `docs/keyboard-map.md` makes and the reason the action was
    /// added rather than the chip being the only way in.
    #[test]
    fn the_browser_key_opens_the_tray_and_shuts_it_again() {
        use bevy::input::ButtonInput;
        use bevy::input::keyboard::KeyboardInput;
        use bevy::prelude::*;

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_message::<KeyboardInput>()
            .init_resource::<crate::Duel>()
            .add_systems(Update, super::keyboard);

        // `reset_all` and not `clear`: a key that is still held is not pressed
        // again, and the second press would fire nothing at all.
        let press = |app: &mut App, key: KeyCode| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);
            app.update();
        };

        assert!(!app.world().resource::<crate::Duel>().browser.is_open());
        press(&mut app, KeyCode::KeyG);
        assert!(
            app.world().resource::<crate::Duel>().browser.is_open(),
            "the browser key did not open the tray"
        );
        press(&mut app, KeyCode::KeyG);
        assert!(
            !app.world().resource::<crate::Duel>().browser.is_open(),
            "it is a latch, so the same key shuts it"
        );
    }

    /// A hold is the one statement a seat makes while it is *not* being asked,
    /// which is also what makes it dangerous: the prompt bar is empty because
    /// the seat is not being asked, and an empty prompt bar is what an idle
    /// turn looks like too. So the key that sets a hold has to be the key that
    /// takes it back, and it has to work from a view alone.
    #[test]
    fn the_hold_keys_stop_the_questions_and_take_it_back() {
        use baylee_engine::choice::PriorityHold;
        use bevy::input::ButtonInput;
        use bevy::input::keyboard::KeyboardInput;
        use bevy::prelude::*;

        use crate::host::{DuelHost, HostMessage, LocalHost};
        let mut host = LocalHost::new(
            &crate::host::tests::duel_preset(),
            PlayerId::new(0),
            &["You", "AI"],
        )
        .expect("host");
        // A real view rather than a hand-built one: `hold_action` reads the
        // turn number and the stack depth off it, and a view assembled by the
        // test would only ever agree with the test.
        let view = host
            .poll()
            .into_iter()
            .find_map(|m| match m {
                HostMessage::View(v) => Some(*v),
                _ => None,
            })
            .expect("a view");
        let turn = view.turn;
        let depth = u16::try_from(view.stack.len()).expect("an opening stack fits");

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::table::CameraRig>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_message::<KeyboardInput>()
            .insert_resource(crate::Duel {
                view: Some(view),
                ..Default::default()
            })
            .add_systems(Update, super::keyboard);

        // `reset_all` and not `clear`: a key still held is not pressed again.
        let press = |app: &mut App, key: KeyCode| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);
            app.update();
        };
        let held = |app: &mut App, held: bool| {
            app.world_mut()
                .resource_mut::<crate::Duel>()
                .view
                .as_mut()
                .expect("the view is still there")
                .priority_held = held;
        };

        press(&mut app, KeyCode::F6);
        press(&mut app, KeyCode::F7);
        assert_eq!(
            app.world().resource::<crate::Duel>().outbox(),
            [
                PlayerAction::SetPriorityHold(PriorityHold::UntilStackEmpty { depth }),
                PlayerAction::SetPriorityHold(PriorityHold::UntilEndOfTurn { turn }),
            ],
            "the two hold keys must say two different things"
        );

        // The engine took it; the view says so. Now either key is the way out,
        // because a player who has stopped being asked should not have to
        // remember which one they pressed.
        held(&mut app, true);
        press(&mut app, KeyCode::F7);
        held(&mut app, true);
        press(&mut app, KeyCode::F6);
        assert_eq!(
            app.world().resource::<crate::Duel>().outbox()[2..],
            [
                PlayerAction::SetPriorityHold(PriorityHold::Always),
                PlayerAction::SetPriorityHold(PriorityHold::Always),
            ],
            "a running hold must be cancelled by either key, never replaced"
        );
    }

    /// The same way out, for a player who never finds a function key.
    #[test]
    fn the_prompt_bar_can_take_a_hold_back_too() {
        use baylee_engine::choice::PriorityHold;

        use crate::host::{DuelHost, HostMessage, LocalHost};
        use crate::hud::MenuAction;
        let mut host = LocalHost::new(
            &crate::host::tests::duel_preset(),
            PlayerId::new(0),
            &["You", "AI"],
        )
        .expect("host");
        let mut view = host
            .poll()
            .into_iter()
            .find_map(|m| match m {
                HostMessage::View(v) => Some(*v),
                _ => None,
            })
            .expect("a view");
        view.priority_held = true;
        let mut duel = crate::Duel {
            view: Some(view),
            ..Default::default()
        };

        super::menu_click(&mut duel, MenuAction::ReleaseHold, false);
        assert_eq!(
            duel.outbox(),
            [PlayerAction::SetPriorityHold(PriorityHold::Always)]
        );

        // And with nothing to release it sends nothing, rather than setting a
        // hold from the button that exists to cancel one.
        duel.view.as_mut().expect("the view").priority_held = false;
        let mut fresh = crate::Duel {
            view: duel.view.clone(),
            ..Default::default()
        };
        super::menu_click(&mut fresh, MenuAction::ReleaseHold, false);
        assert!(fresh.outbox().is_empty());
    }

    /// A duel driven to seat 0's first main phase, out of a real `LocalHost`.
    ///
    /// Every one of the arming tests needs a `LegalActions` that actually
    /// offers something, and a hand-built one offers whatever the test wanted
    /// it to. This plays the opening the way the client does — keep the hand,
    /// pass until the engine offers a land — and stops at the window where a
    /// player has a decision to make.
    fn duel_in_main_phase() -> (crate::Duel, crate::host::LocalHost) {
        use crate::host::{DuelHost, HostMessage, LocalHost};
        use baylee_engine::choice::Pending;

        let seat = PlayerId::new(0);
        let mut host = LocalHost::new(&crate::host::tests::duel_preset(), seat, &["You", "AI"])
            .expect("the preset makes a game");
        let mut duel = crate::Duel::default();
        for _ in 0..64 {
            for message in host.poll() {
                match message {
                    HostMessage::Static(s) => duel.statics = Some(*s),
                    HostMessage::View(v) => duel.view = Some(*v),
                    HostMessage::Choice(p) => {
                        duel.interaction = Some(Interaction::new(*p, seat));
                    }
                    HostMessage::Failed(why) => panic!("the host refused: {why}"),
                }
            }
            match duel.interaction.as_ref().map(Interaction::pending) {
                Some(Pending::Mulligan { .. }) => host.submit(PlayerAction::MulliganKeep),
                Some(Pending::Priority { legal, .. }) if !legal.lands.is_empty() => {
                    return (duel, host);
                }
                Some(Pending::Priority { .. }) => host.submit(PlayerAction::PassPriority),
                _ => break,
            }
        }
        panic!("the game never offered seat 0 a land to play");
    }

    /// The whole of arm-then-act: the first tap says nothing, the second
    /// sends, and cancel leaves the wire empty.
    ///
    /// A window offering one land, one spell, and one card that is both.
    ///
    /// Synthetic rather than dealt, because what is under test is the click
    /// path and a real deck cannot be relied on to hold a castable spell and a
    /// modal double-faced land in the same opening hand.
    fn window_with(lands: Vec<ObjectId>, castable: Vec<ObjectId>) -> crate::Duel {
        crate::Duel {
            interaction: Some(Interaction::new(
                Pending::Priority {
                    player: PlayerId::new(0),
                    legal: Box::new(LegalActions {
                        can_pass: true,
                        lands,
                        castable,
                        mana_abilities: vec![],
                        abilities: vec![],
                        suspendable: vec![],
                    }),
                },
                PlayerId::new(0),
            )),
            ..Default::default()
        }
    }

    /// A tap on the top card of a pile opens that pile.
    ///
    /// This is the only way in to a graveyard now that the pile chips are
    /// gone, so it needs a witness rather than a reading: `open_pile` was
    /// wired before the chips were removed and nothing ever clicked it, which
    /// is precisely the shape of defect this client has shipped before. The
    /// door has to be proved from a *tap* — `activate_card`, the same
    /// function the pointer calls — and not by calling `open_pile` directly,
    /// or the test would pass with the last branch of `activate_card` gone.
    #[test]
    fn a_tap_on_a_pile_opens_it() {
        use baylee_client_core::test_support::{ViewBuilder, printed};

        let top = obj(7);
        let view = ViewBuilder::new(2)
            .with_graveyard(0, vec![printed(7, 0, "Llanowar Elves", 1)])
            .build();
        let mut duel = crate::Duel::default();
        duel.receive_view(view);
        crate::rebuild_board(&mut duel);
        assert!(!duel.browser.is_open(), "nothing has been tapped yet");

        super::activate_card(&mut duel, top);
        assert!(duel.browser.is_open(), "the graveyard did not open");
        assert_eq!(
            duel.browser.tab(),
            Some(baylee_client_core::BrowseZone::Graveyard(PlayerId::new(0))),
            "it opened on somebody else's pile"
        );
    }

    /// And a library never opens, however often it is tapped: nobody may look
    /// through one, their own included (CR 401.2), so the pile beside the mat
    /// is inert rather than merely empty.
    ///
    /// The pile is **manufactured**, because a view cannot produce one: a
    /// library is face down to everybody, so `ZonePile::top` is always `None`
    /// there and no tap can ever name its card. It is built anyway because a
    /// test that tapped an object on no pile at all would pass with every
    /// guard in `open_pile` deleted, and would then be claiming CR 401.2 while
    /// holding nothing. Two independent readings refuse it — `is_browsable`
    /// and `BrowseZone::of_pile` — and each has its own witness in
    /// `baylee-client-core`; what is asserted here is the outcome a player
    /// sees.
    #[test]
    fn a_tap_on_a_library_opens_nothing() {
        use baylee_client_core::layout::PileKind;
        use baylee_client_core::test_support::ViewBuilder;

        let top = obj(7);
        let view = ViewBuilder::new(2).build();
        let mut duel = crate::Duel::default();
        duel.receive_view(view);
        crate::rebuild_board(&mut duel);
        {
            let board = duel.board.as_mut().expect("the view built a board");
            let pile = board.pods[0]
                .piles
                .iter_mut()
                .find(|pile| pile.kind == PileKind::Library)
                .expect("every seat has a library");
            pile.count = 60;
            pile.top = Some(top);
        }

        super::activate_card(&mut duel, top);
        assert!(!duel.browser.is_open());
    }

    #[test]
    fn a_land_plays_on_the_click() {
        let land = obj(3);
        let mut duel = window_with(vec![land], vec![]);

        super::activate_card(&mut duel, land);
        assert_eq!(
            duel.outbox(),
            [PlayerAction::PlayLand { card: land }],
            "the land did not play on the click"
        );
        assert!(duel.armed.is_none(), "a land asked for a confirmation");
    }

    /// There is no undo in the engine and there should not be one, so the
    /// client owes a player the chance to take a tap back before it becomes a
    /// game action. Tested at this level and not on `Interaction`, because
    /// what is being claimed is about *taps*: an assertion that the second
    /// call to a resolver returns an action would pass just as well if the
    /// first one had already sent it.
    #[test]
    fn a_spell_still_arms_and_a_second_tap_sends_it() {
        let spell = obj(4);
        let mut duel = window_with(vec![], vec![spell]);

        super::activate_card(&mut duel, spell);
        assert!(
            duel.outbox().is_empty(),
            "the first tap put a spell on the wire"
        );
        assert_eq!(
            duel.armed,
            Some(crate::Armed {
                object: spell,
                deed: crate::Deed::Play
            })
        );

        super::activate_card(&mut duel, spell);
        assert_eq!(duel.outbox(), [PlayerAction::CastSpell { card: spell }]);
        assert!(duel.armed.is_none(), "firing left the deed armed");
    }

    /// The exception to the exception. A modal double-faced card with a spell
    /// front and a land back is in *both* lists, and `play_card` checks lands
    /// first — so one-clicking it would resolve it to "play as land" every
    /// time and the front face would be unreachable by mouse.
    #[test]
    fn a_card_that_is_both_a_land_and_a_spell_does_not_one_click() {
        let mdfc = obj(5);
        let mut duel = window_with(vec![mdfc], vec![mdfc]);

        super::activate_card(&mut duel, mdfc);
        assert!(
            duel.outbox().is_empty(),
            "a card with two ways to play it fired one of them on the click"
        );
        assert!(duel.armed.is_some());
    }

    /// Cancel is the whole point of arming: it has to leave nothing behind.
    #[test]
    fn cancel_disarms_with_nothing_on_the_wire() {
        use crate::keys::Fired;
        use baylee_client_core::prefs::{Action, Chord, Keymap};
        use bevy::prelude::KeyCode;

        // A spell, because a land no longer arms at all.
        let spell = obj(4);
        let mut duel = window_with(vec![], vec![spell]);

        super::activate_card(&mut duel, spell);
        assert!(duel.armed.is_some());

        let mut keymap = Keymap::standard();
        keymap.bind(Action::Cancel, vec![Chord::key("Escape")]);
        let mut keys = bevy::input::ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Escape);
        assert!(super::armed_keys(Fired::of(&keys, &keymap), &mut duel));
        assert!(duel.armed.is_none(), "cancel left the deed armed");
        assert!(duel.outbox().is_empty(), "cancel sent something");
    }

    /// The exception, and the reason it is one: floating mana is the cheap
    /// mistake in this game, so tapping a land stays a single tap.
    #[test]
    fn a_mana_ability_still_goes_through_on_one_tap() {
        use crate::host::DuelHost;
        let (mut duel, mut host) = duel_in_main_phase();
        let land = duel
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
            .and_then(|l| l.lands.first().copied())
            .expect("the window offers a land");

        // Put the land in play first — it is the only mana source this deck
        // has, and a land in hand makes no mana. One call: a land plays on
        // the click now, and a second would send `PlayLand` twice.
        super::activate_card(&mut duel, land);
        // `outbox` is private to `Duel` but declared in the crate root, so a
        // child module may drain it — which is what `flush_outbox` does in
        // the running client.
        for action in std::mem::take(&mut duel.outbox) {
            host.submit(action);
        }
        for message in host.poll() {
            match message {
                crate::host::HostMessage::View(v) => duel.view = Some(*v),
                crate::host::HostMessage::Choice(p) => {
                    duel.interaction = Some(Interaction::new(*p, PlayerId::new(0)));
                }
                _ => {}
            }
        }
        let source = duel
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
            .and_then(|l| l.mana_abilities.first().copied())
            .expect("the land that was just played can be tapped");

        super::activate_card(&mut duel, source);
        assert!(
            duel.armed.is_none(),
            "tapping for mana asked for a confirmation"
        );
        assert_eq!(
            duel.outbox(),
            [PlayerAction::ActivateManaAbility { source }]
        );
    }
}

#[cfg(test)]
mod refusal_tests {
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_engine::choice::{LegalActions, Pending, PlayerAction};

    fn priority() -> Pending {
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                ..LegalActions::default()
            }),
        }
    }

    /// A refusal stands until this seat tries something else.
    ///
    /// It cannot be cleared when the next question arrives, which is the
    /// obvious place: the acting seat is re-sent its own question every time
    /// anybody at the table says anything, so the line would have been wiped
    /// a frame or two after it appeared — and the click that earned it is the
    /// click the player is waiting to understand.
    #[test]
    fn a_refusal_outlives_the_question_it_answered() {
        let mut duel = crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                priority(),
                PlayerId::new(0),
            )),
            last_error: Some("illegal action for your seat".to_string()),
            ..crate::Duel::default()
        };

        // The same question again — an opponent said something. This goes
        // through the arm that installs it, because assigning the field would
        // only be testing that writing one field leaves another alone.
        duel.receive_choice(priority());
        assert_eq!(
            duel.last_error.as_deref(),
            Some("illegal action for your seat"),
            "a re-sent question is not this player doing anything"
        );

        // This player tries again: the old refusal is about the old attempt.
        duel.submit(PlayerAction::PassPriority);
        assert!(duel.last_error.is_none());
        assert_eq!(duel.outbox(), &[PlayerAction::PassPriority]);
    }

    /// An armed deed the engine has withdrawn says so, and says it without
    /// sending anything.
    #[test]
    fn a_stale_deed_reports_instead_of_firing() {
        let mut duel = crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                priority(),
                PlayerId::new(0),
            )),
            armed: Some(crate::Armed {
                object: ObjectId::new(3, 0),
                deed: crate::Deed::Play,
            }),
            ..crate::Duel::default()
        };
        super::fire_armed(&mut duel);
        assert!(duel.outbox().is_empty(), "nothing goes on the wire");
        assert_eq!(duel.last_error.as_deref(), Some(super::STALE));
    }
}

/// The zone browser's sheet actually moves when its header is dragged.
///
/// A drag cannot be proved through the `dev-control` harness — `/pointer`
/// presses and releases in one call, so there is no frame in the middle where
/// the cursor is somewhere else — and "declared but never wired" is a bug this
/// client has shipped before. So it is proved here instead: the system is put
/// in an `App` with a window, a sheet and the two markers, and what is
/// asserted is the sheet's own `Node`, not that `update()` returned.
#[cfg(test)]
mod dragging {
    use super::*;
    use crate::hud::{TrayGrip, TrayPanel, TrayResize};
    use bevy::picking::events::{Pointer, Press, Release};
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::window::{PrimaryWindow, WindowRef};

    /// A window with a sheet in it and nothing else.
    ///
    /// The window is given a cursor because the system reads one: a press
    /// with no cursor position starts no drag, which is the branch that keeps
    /// a harness that never moved a mouse from throwing the sheet across the
    /// band.
    fn harness() -> (App, Entity, Entity, Entity) {
        let mut app = App::new();
        app.add_message::<Pointer<Press>>()
            .add_message::<Pointer<Release>>()
            .init_resource::<Duel>()
            .init_resource::<ClientSettings>()
            .add_systems(Update, tray_drag);
        let mut window = Window::default();
        window.resolution.set(1728.0, 1052.0);
        window.set_cursor_position(Some(Vec2::new(800.0, 400.0)));
        let win = app.world_mut().spawn((window, PrimaryWindow)).id();
        // The node the overlay would have built: an explicit rectangle, so
        // that "it moved" is a comparison of two numbers rather than of a
        // number against `Auto`.
        let band = (1728.0, 1052.0 - crate::hud::EDGE - crate::hud::HAND_BAR_H);
        let home = Placement::centred(band);
        let panel = app
            .world_mut()
            .spawn((
                TrayPanel,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(home.left),
                    top: px(home.top),
                    width: px(home.width),
                    height: px(home.height),
                    ..default()
                },
            ))
            .id();
        let grip = app.world_mut().spawn((TrayGrip, Node::default())).id();
        let corner = app.world_mut().spawn((TrayResize, Node::default())).id();
        app.world_mut()
            .entity_mut(panel)
            .add_children(&[grip, corner]);
        let _ = win;
        (app, panel, grip, corner)
    }

    /// Where the pointer is, as the picking backend would report it.
    fn at(app: &mut App, position: Vec2) -> Location {
        use bevy::camera::NormalizedRenderTarget;
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .expect("the harness made a window");
        let target = WindowRef::Entity(window)
            .normalize(Some(window))
            .expect("a window is a render target");
        Location {
            target: NormalizedRenderTarget::Window(target),
            position,
        }
    }

    fn press(app: &mut App, entity: Entity) {
        let location = at(app, Vec2::ZERO);
        let event = Press {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
            count: 1,
        };
        app.world_mut()
            .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
    }

    fn release(app: &mut App, entity: Entity) {
        let location = at(app, Vec2::ZERO);
        let event = Release {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
        };
        app.world_mut()
            .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
    }

    /// Moves the cursor and runs one frame.
    fn cursor_to(app: &mut App, position: Vec2) {
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .expect("the harness made a window");
        app.world_mut()
            .entity_mut(window)
            .get_mut::<Window>()
            .expect("a window")
            .set_cursor_position(Some(position));
        app.update();
    }

    fn node_of(app: &App, panel: Entity) -> Node {
        app.world()
            .entity(panel)
            .get::<Node>()
            .expect("a node")
            .clone()
    }

    #[test]
    fn dragging_the_header_moves_the_sheet() {
        let (mut app, panel, grip, _) = harness();
        press(&mut app, grip);
        app.update();
        let before = node_of(&app, panel);

        cursor_to(&mut app, Vec2::new(860.0, 430.0));
        let after = node_of(&app, panel);
        assert_ne!(after.left, before.left, "the sheet did not move sideways");
        assert_ne!(after.top, before.top, "the sheet did not move down");
        assert_eq!(after.width, before.width, "a drag resized it");

        // The position is remembered, and it is the *settings* that hold it
        // rather than the node — so a rebuild for any other reason lands
        // where the pointer left it rather than back in the middle.
        assert!(
            app.world()
                .resource::<ClientSettings>()
                .zone_browser
                .is_some(),
            "nothing was remembered"
        );

        // And a release lets go: the sheet stops following the pointer.
        release(&mut app, grip);
        app.update();
        let parked = node_of(&app, panel);
        cursor_to(&mut app, Vec2::new(300.0, 200.0));
        assert_eq!(
            node_of(&app, panel).left,
            parked.left,
            "the sheet is still following a pointer nobody is holding"
        );
    }

    #[test]
    fn dragging_the_corner_stretches_the_sheet() {
        let (mut app, panel, _, corner) = harness();
        press(&mut app, corner);
        app.update();
        let before = node_of(&app, panel);

        cursor_to(&mut app, Vec2::new(900.0, 500.0));
        let after = node_of(&app, panel);
        assert_ne!(after.width, before.width, "the corner did not stretch it");
        assert_eq!(after.left, before.left, "the corner moved the sheet");
    }

    /// Clicking the corner fills the band, and clicking it again puts the
    /// sheet back where it was.
    ///
    /// The corner draws a ⤢ and is read as a maximise button, and it was a
    /// drag handle and nothing else — a click on it did nothing at all, which
    /// is what the owner reported. The two gestures share one control, so the
    /// third case is the one that matters: a *drag* must not maximise.
    #[test]
    fn clicking_the_corner_maximises_and_restores_the_sheet() {
        let (mut app, panel, _, corner) = harness();
        let band = (1728.0, 1052.0 - crate::hud::EDGE - crate::hud::HAND_BAR_H);
        let home = node_of(&app, panel);

        press(&mut app, corner);
        app.update();
        release(&mut app, corner);
        app.update();
        let full = node_of(&app, panel);
        assert_ne!(full.width, home.width, "the click did nothing");
        assert_eq!(full.width, px(Placement::maximised(band).width));
        assert_eq!(full.height, px(Placement::maximised(band).height));

        press(&mut app, corner);
        app.update();
        release(&mut app, corner);
        app.update();
        let back = node_of(&app, panel);
        assert_eq!(back.width, home.width, "it did not go back");
        assert_eq!(back.left, home.left, "it went back somewhere else");
    }

    /// And a drag on the corner is a resize, never a maximise.
    #[test]
    fn dragging_the_corner_does_not_maximise_it() {
        let (mut app, panel, _, corner) = harness();
        let band = (1728.0, 1052.0 - crate::hud::EDGE - crate::hud::HAND_BAR_H);
        press(&mut app, corner);
        app.update();
        cursor_to(&mut app, Vec2::new(900.0, 500.0));
        let stretched = node_of(&app, panel);
        release(&mut app, corner);
        app.update();

        assert_eq!(
            node_of(&app, panel).width,
            stretched.width,
            "letting go of a resize maximised the sheet"
        );
        assert_ne!(
            node_of(&app, panel).width,
            px(Placement::maximised(band).width)
        );
    }

    /// The ✕ stands *on* the header, and pressing it must not start a move.
    ///
    /// It would be a slow leak rather than a visible bug: a hand that wobbles
    /// a pixel between the press and the release moves the sheet a pixel, the
    /// release saves it, and the sheet creeps a little further from where it
    /// was put with every close.
    #[test]
    fn pressing_the_close_button_does_not_start_a_drag() {
        let (mut app, panel, grip, _) = harness();
        let close = app.world_mut().spawn((TrayClose, Node::default())).id();
        app.world_mut().entity_mut(grip).add_children(&[close]);

        press(&mut app, close);
        app.update();
        let before = node_of(&app, panel);
        cursor_to(&mut app, Vec2::new(1000.0, 600.0));
        assert_eq!(node_of(&app, panel).left, before.left);
        assert!(
            app.world()
                .resource::<ClientSettings>()
                .zone_browser
                .is_none(),
            "closing the sheet wrote a place nobody chose"
        );
    }

    /// A press somewhere else is not a drag.
    #[test]
    fn pressing_the_sheet_itself_does_not_move_it() {
        let (mut app, panel, _, _) = harness();
        press(&mut app, panel);
        app.update();
        let before = node_of(&app, panel);
        cursor_to(&mut app, Vec2::new(1000.0, 600.0));
        assert_eq!(node_of(&app, panel).left, before.left);
    }

    /// The sheet a question opened is not furniture, and cannot be rearranged.
    ///
    /// The half that is easy to miss is the *writing*: `Browser::placement`
    /// already refuses to read a stored rectangle for such a sheet, so a drag
    /// that still wrote one would move the panel under the hand, snap it back
    /// to the middle at the next rebuild, and leave the remembered place of
    /// the hand-opened sheet somewhere nobody chose.
    #[test]
    fn a_sheet_a_question_opened_cannot_be_dragged() {
        use baylee_client_core::test_support::ViewBuilder;
        use baylee_core::ids::PlayerId;
        use baylee_engine::choice::{ChoicePrompt, Pending};

        let (mut app, panel, grip, _) = harness();
        let view = ViewBuilder::new(2).build();
        let asked = Pending::ChooseCards {
            player: PlayerId::new(0),
            options: vec![ObjectId::new(7, 0)],
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SearchLibrary,
        };
        let interaction =
            baylee_client_core::interaction::Interaction::new(asked, PlayerId::new(0));
        app.world_mut()
            .resource_mut::<Duel>()
            .browser
            .follow(&view, Some(&interaction));
        assert!(
            app.world().resource::<Duel>().browser.for_choice(),
            "the harness did not open the sheet for a question"
        );

        press(&mut app, grip);
        app.update();
        let before = node_of(&app, panel);
        cursor_to(&mut app, Vec2::new(1200.0, 700.0));
        assert_eq!(
            node_of(&app, panel).left,
            before.left,
            "a question's sheet followed the pointer"
        );
        assert!(
            app.world()
                .resource::<ClientSettings>()
                .zone_browser
                .is_none(),
            "a question's sheet wrote a place the player will never see it in"
        );
    }
}
