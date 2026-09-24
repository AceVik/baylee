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
    PromptAction, PromptButton, TrayCard, TrayFilter, TrayMinimise, TrayNone, TraySort, TrayTab,
};
use crate::keys::Fired;
use crate::settings::ClientSettings;
use crate::table::CardVisual;
use crate::{Deed, Duel, HoverSpot};
use baylee_client_core::abilitysheet;
use baylee_client_core::arrange::{Arrangement, Nudge};
use baylee_client_core::automation::AutoPilot;
use baylee_client_core::browser::Placement;
use baylee_client_core::filterdialog::FilterPanel;
use baylee_client_core::i18n::{Phrase, Refusal};
use baylee_client_core::interaction::{Interaction, Prompt, SelectionOutcome};
use baylee_client_core::prefs::Action;
use baylee_client_core::touch::Answer;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::PlayerAction;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

/// Every widget the zone browser puts on screen, as one system parameter.
///
/// Bundled rather than four more arguments because `pointer` already sits at
/// Bevy's parameter limit — and because they are one thing: the tray, its
/// tabs, its close button, its filter box and its sort control.
///
/// The settings store rides along for the same reason and is the one member
/// that is not a query: the view buttons are the only control on this panel
/// whose state does not live on the `Browser`, and `pointer` has no room left
/// to be handed it separately.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TrayWidgets<'w, 's> {
    cards: Query<'w, 's, &'static TrayCard>,
    /// The end of each pile of an arrangement, while a card is held.
    slots: Query<'w, 's, &'static crate::hud::ArrangeSlot>,
    tabs: Query<'w, 's, &'static TrayTab>,
    close: Query<'w, 's, &'static TrayMinimise>,
    /// The tray's own button, which is not on the sheet at all — it
    /// stands on the ledge whether the sheet is up or down. It is in
    /// this bundle rather than beside it because what it operates is
    /// this panel, and a reader looking for who opens the browser
    /// should find every door in one place.
    zones: Query<'w, 's, &'static crate::hud::TrayZones>,
    sort: Query<'w, 's, &'static TraySort>,
    views: Query<'w, 's, &'static crate::hud::TrayView>,
    filter: Query<'w, 's, &'static TrayFilter>,
    gear: Query<'w, 's, &'static crate::hud::TrayGear>,
    /// Every button the filter builder draws, in one query.
    ///
    /// One and not fourteen, because the model already named what each one
    /// means: `crate::filterui` puts a `FilterAct` on every button it draws
    /// and this hands it straight to the `Browser`. A control added to the
    /// builder is therefore wired by being drawn, which is the whole point of
    /// the vocabulary — this client has shipped a decision function no button
    /// reached.
    acts: Query<'w, 's, &'static crate::filterui::FilterAct>,
    done: Query<'w, 's, &'static crate::filterui::FilterDone>,
    cancel: Query<'w, 's, &'static TrayNone>,
    settings: ResMut<'w, crate::settings::ClientSettings>,
    /// The sheet's other size button, and the two things it takes to answer
    /// it. A placement is only meaningful against the band it was measured
    /// in, so the window comes with the button rather than being fetched by
    /// whoever happens to need it — `Placement::maximised`, `is_maximised`
    /// and `fit` all take one, and a band read from somewhere else is a sheet
    /// that maximises to the wrong rectangle.
    grow: Query<'w, 's, &'static crate::hud::TrayMaximise>,
    windows: Query<'w, 's, &'static Window>,
    glide: ResMut<'w, TrayGlide>,
}

/// The sheet on its way between two rectangles.
///
/// Maximising and restoring used to be one assignment — `settings.zone_browser
/// = Some(next)` and the sheet was simply *there* on the next frame. The owner
/// asked for the movement on 19.09.2026: *"Das Maximiere und reverse soll auch
/// schön animiert sein"*.
///
/// What travels is the **rectangle**, not a `UiTransform`. A transform scales
/// what is already laid out, so a sheet stretched from 900×738 to the band's
/// shape would carry its type, its thumbnails and its row heights with it and
/// arrive as a distorted picture that snaps straight at the end. Writing
/// `Placement::lerp` into the `Node` re-lays the sheet out on every frame,
/// which is what makes the rows stay rows the whole way across.
///
/// The target is written to the store on the **first** frame, not the last:
/// `settings.zone_browser` is where a rebuild reads the sheet's rectangle
/// from, and a store that still held the old one would put the sheet back
/// where it started if a card arrived mid-flight. So the store says where the
/// sheet is going and this says where it is — the same split `tray_drag`
/// already keeps, and the reason `write_placement` exists at all.
/// It is one `Option` and not three fields with a flag among them, because
/// "nothing is moving" has no *from* and no *to* — a resting glide holding
/// two rectangles would need a zero `Placement` that is not a rectangle any
/// sheet could be at, and `Placement` deliberately has no `Default` for that
/// reason.
#[derive(Resource, Default)]
pub struct TrayGlide(Option<Flight>);

impl TrayGlide {
    /// Whether the sheet is between two rectangles right now.
    ///
    /// The one thing about this resource anybody outside needs to ask, and
    /// the reason it is asked at all: "the store holds the band" and "the
    /// sheet is on its way to the band" are two different claims, and a test
    /// that only checked the first would pass on the jump this movement
    /// replaced.
    #[must_use]
    pub fn is_flying(&self) -> bool {
        self.0.is_some()
    }
}

/// One movement between two rectangles.
struct Flight {
    /// Where the sheet set off from.
    from: Placement,
    /// Where it is going, which is also what the store already holds.
    to: Placement,
    /// 0 at the start of the movement, 1 at its end.
    t: f32,
}

/// Everything on the ability sheet a pointer can land on, bundled for the
/// same reason [`TrayWidgets`] is: `pointer` is at Bevy's parameter limit,
/// and these three are one surface. A row arms, the tenth row turns the
/// page, the cross in the head is the way out.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SheetWidgets<'w, 's> {
    rows: Query<'w, 's, &'static AbilityButton>,
    pager: Query<'w, 's, &'static crate::hud::SheetPager>,
    close: Query<'w, 's, &'static crate::hud::SheetClose>,
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
    if select_stack_stop(duel, object) {
        return Answer::Took;
    }
    // A second tap on the same card is the send. A tap on a different one is
    // a change of mind and not a confirmation, so it disarms and arms afresh.
    if duel.armed.as_ref().is_some_and(|a| a.object == object) {
        fire_armed(duel);
        return Answer::Took;
    }
    duel.armed = None;
    // A card with more than one way to be cast is asked about **first**, and
    // that ordering is the whole of AZ. Both branches under this one commit
    // to a way without saying so: the engine offers a card in `castable` only
    // for the ways the *current* pool pays for — which for Solitude with an
    // empty pool is the free evoke and nothing else — and the run below
    // floats exactly the printed cost, after which the evoke is the way that
    // has become unaffordable. Either way the player was told nothing.
    // [`crate::castmodes`] is what may be asked about.
    if duel.cast_menu.as_ref().is_some_and(|m| m.card == object) {
        // A second tap on the card whose chooser is already open does
        // nothing, the same as the ability sheet's: it is the player's own
        // question standing, not a button that failed.
        return Answer::Took;
    }
    // A tap on a *different* card puts the chooser away, which is the same
    // change of mind that disarmed above. Every branch below reaches a card
    // that opens no chooser of its own — a land, a permanent with abilities,
    // a spell with one way — and each of them left the old card's question
    // standing over the new card's deed: one headline asking how to cast
    // Solitude, one row offering to play Reveillark, and a row press that
    // armed the card the player had stopped looking at.
    duel.cast_menu = None;
    // And the way chosen for that other card goes with it. The answer
    // deliberately outlives the run that spends it — a free alternative has
    // no run at all — so the gestures that *are* a player leaving a card have
    // to say so, or the engine's next `ChooseCastMode` about it is answered
    // by a decision that was walked away from.
    if duel.cast_answer.is_some_and(|(card, _)| card != object) {
        duel.cast_answer = None;
    }
    if let Some(menu) = cast_menu_for(duel, object) {
        duel.ability_menu = None;
        duel.cast_menu = Some(menu);
        return Answer::Took;
    }
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
    // suspend card there prints no mana cost, so CR 202.1b keeps it out of
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
                // A second tap on the card whose sheet is already open does
                // **nothing**, rather than putting the cursor and the page
                // back to the top of a list the player is reading. The card
                // is one of the two places a click does not close the sheet
                // (see `close_the_sheet_on_a_press_outside_it`), so without
                // this it would be the one place that answers a click by
                // losing the reader's place.
                if duel.ability_menu != Some(object) {
                    duel.ability_menu = Some(object);
                    duel.ability_pick = 0;
                    duel.ability_page = 0;
                    // A fresh sheet is the whole list, never a tap somebody
                    // stepped into on the card before it.
                    duel.ability_tap = None;
                }
                return Answer::Took;
            }
        }
    }
    duel.ability_menu = None;
    if answer_with(duel, object) || open_pile(duel, object) {
        Answer::Took
    } else {
        if let Some(reason) = hand_refusal(duel, object) {
            duel.last_error = Some(baylee_client_core::i18n::Refusal::Said(reason));
        }
        Answer::Refused
    }
}

/// Puts a card into the answer being built, or takes it out.
///
/// A card on the table can stand for several, and a click on it means one
/// more of them — or one fewer, on a card of ones already chosen — rather
/// than the one it happens to be drawn as. Until #210 this toggled the drawn
/// one only, so the second click on twelve Soldiers took back the first
/// instead of sending a second.
fn answer_with(duel: &mut Duel, object: ObjectId) -> bool {
    let members = members_of(duel, object);
    duel.interaction
        .as_mut()
        .is_some_and(|i| i.toggle_group(&members) != SelectionOutcome::Rejected)
}

/// Everything a card on the table stands for, or the object alone when it
/// is not a battlefield card (a hand card, a pile's top, a stack entry).
fn members_of(duel: &Duel, object: ObjectId) -> Vec<ObjectId> {
    duel.board
        .as_ref()
        .and_then(|board| board.group(object))
        .map_or_else(|| vec![object], |group| group.members.clone())
}

/// [`activate_card`], or with `whole` the same gesture meant for the whole
/// card: every permanent it stands for, at once (`Interaction::toggle_all`).
///
/// Only a choice being answered has a whole card to take — a declaration,
/// targets, a sacrifice. Anywhere else a card is one card, and the gesture
/// is an ordinary tap on it.
pub fn activate(duel: &mut Duel, object: ObjectId, whole: bool) -> Answer {
    let members = members_of(duel, object);
    let answered = whole
        && members.len() > 1
        && duel.interaction.as_mut().is_some_and(|i| {
            i.legal_actions().is_none() && i.toggle_all(&members) != SelectionOutcome::Rejected
        });
    if answered {
        duel.ability_menu = None;
        return Answer::Took;
    }
    activate_card(duel, object)
}

/// Explain a refused hand-card gesture using facts visible to this seat.
fn hand_refusal(duel: &Duel, object: ObjectId) -> Option<Phrase> {
    let view = duel.view.as_ref()?;
    let card = view.hand.iter().find(|card| card.id == object)?;
    if crate::targeting::provably_targetless(view, card.card) {
        return Some(Phrase::CardHasNoTarget);
    }
    let priority = duel.interaction.as_ref().is_some_and(|i| {
        matches!(
        i.pending(), baylee_engine::choice::Pending::Priority { player, .. }
            if *player == view.seat)
    });
    let flash = baylee_cards::by_index(card.card.index).is_some_and(|def| {
        def.keywords.contains(baylee_cards_dsl::KeywordSet::FLASH)
            || def
                .faces
                .get(usize::from(card.card.face))
                .is_some_and(|face| face.keywords.contains(baylee_cards_dsl::KeywordSet::FLASH))
    });
    Some(
        if !priority || !baylee_client_core::timing::allows(view, card.types, flash) {
            Phrase::CardWrongTime
        } else {
            Phrase::CardCostsUnavailable
        },
    )
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

/// The chooser for `object`, when it has more than one way to be cast.
///
/// `None` is the ordinary answer and means "carry on as before": one way, no
/// way, or a card this client would not offer to pay for anyway.
///
/// The two membership tests are not a third judgement about timing and the
/// board — they are the two this client already makes, asked again. A card
/// the engine offers is castable now; a card in `reachable` has been through
/// `timing::allows` and has a plan. Without them a sorcery with two printed
/// ways would open a chooser on an opponent's turn and every row in it would
/// lead nowhere.
fn cast_menu_for(duel: &Duel, object: ObjectId) -> Option<crate::CastMenu> {
    let view = duel.view.as_ref()?;
    let legal = duel.interaction.as_ref()?.legal_actions()?;
    if !legal.castable.contains(&object) && !duel.reachable.contains(&object) {
        return None;
    }
    let modes = crate::castmodes::reachable_modes(view, legal, object);
    (modes.len() > 1).then_some(crate::CastMenu {
        card: object,
        modes,
        pick: 0,
    })
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
/// while a row is armed — the keycap goes gilt and the same digit again
/// sends it — so the caller needs to know which of the two happened, and a
/// sheet that closed on the arming would take the digit away with it.
fn arm_ability(
    duel: &mut Duel,
    object: ObjectId,
    option: &crate::abilities::AbilityOption,
) -> bool {
    // A pip of a mana bubble, and this is the press the owner asked for. The
    // permanent has not been touched yet: the colour was chosen on a bubble
    // that cost nothing to open, and the two go out together as a one-step
    // run — the activation now, the engine's `ChooseColor` answered from
    // `ManaRun::asking` on the frame after. A `duel.submit` here would be the
    // old path exactly, which is tap first and ask second.
    if let Some(pour) = option.pour {
        duel.last_error = None;
        duel.armed = None;
        duel.mana_run = Some(crate::ManaRun::new(
            baylee_client_core::manaplan::Plan {
                steps: vec![pour.step],
                ..Default::default()
            },
            object,
            crate::RunEnd::Float,
        ));
        return true;
    }
    // A **written** mana row: the pips could not stand for this tap, because
    // what one press of it pours is a number this side cannot count. Sending
    // it would tap the card and hand the colour question to the engine's own
    // chooser; what the owner asked for is the mana dialog, so the press
    // steps into the tap and the sheet becomes that tap's bubble.
    //
    // Nothing is on the wire yet, which is the same bargain a bubble has
    // always struck: the card is tapped by the press that answers, not by the
    // press that asked.
    //
    // A step is only worth taking into a bubble that is a **question**, which
    // is why the pips are built here and counted before the sheet turns into
    // one. Two shapes would otherwise walk into a dead end: a tap that pours
    // an uncountable amount of *one* colour (`Effect::mana_dynamic` — "add
    // {G} for each Ally") has exactly one answer, and a tap whose colours no
    // board can resolve has none at all, so the sheet would become a bubble
    // with nothing in it to press. Neither is printed on a card in this pool
    // today; the rule `docs/client.md` states is general, and a row that
    // cannot ask anything is sent the way it always was.
    if option.mana
        && let PlayerAction::ActivateAbility {
            source,
            ability_index,
        } = option.action
        && source == object
        && let Some(view) = duel.view.as_ref()
        && let Some(interaction) = duel.interaction.as_ref()
        && !crate::manasources::countable(
            view,
            object,
            baylee_client_core::manaplan::Tap::Ability(ability_index),
        )
        && crate::abilities::options_for(
            baylee_client_core::Lang::En,
            view,
            interaction,
            object,
            Some(ability_index),
        )
        .len()
            > 1
    {
        duel.last_error = None;
        duel.armed = None;
        duel.ability_menu = Some(object);
        duel.ability_tap = Some(ability_index);
        duel.ability_pick = 0;
        duel.ability_page = 0;
        return false;
    }
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
    // The cast chooser is already closed by the press that picked a row; this
    // is the case where a deed was armed some other way while one stood.
    duel.cast_menu = None;
    match armed.deed {
        Deed::Play => {
            match duel
                .interaction
                .as_ref()
                .and_then(|i| i.play_card(armed.object))
            {
                Some(action) => duel.submit(action),
                None => duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn)),
            }
        }
        Deed::Ability(action) => {
            let still_offered = abilities_of(duel, armed.object)
                .is_some_and(|options| options.iter().any(|o| o.action == action));
            if still_offered {
                duel.submit(action);
            } else {
                duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn));
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
                None => duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn)),
            }
        }
        Deed::Run {
            plan,
            then: crate::RunEnd::Cast,
        } => {
            // The short-circuit below is for the *manual* land tap that may
            // have happened between the two clicks, and it must not fire when
            // the player has chosen a way to cast: the engine's answer with
            // the pool as it stands is exactly the wrong one — a Solitude
            // whose printed cost the player picked is `castable` this whole
            // time, for its free evoke. So a chosen way with taps left to
            // make goes to the run, and only the run.
            let chosen = duel
                .cast_answer
                .as_ref()
                .is_some_and(|(card, _)| *card == armed.object);
            if chosen && !plan.is_empty() {
                duel.last_error = None;
                duel.mana_run = Some(crate::ManaRun::new(plan, armed.object, crate::RunEnd::Cast));
            } else if let Some(action) = duel
                .interaction
                .as_ref()
                .and_then(|i| i.play_card(armed.object))
            {
                duel.submit(action);
            } else if duel.reachable.contains(&armed.object) {
                duel.last_error = None;
                duel.mana_run = Some(crate::ManaRun::new(plan, armed.object, crate::RunEnd::Cast));
            } else {
                duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn));
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
                duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn));
            }
        }
        // A pour is never armed — [`arm_ability`] starts its run on the press
        // that picks the colour, which is the whole of "the card taps once
        // the mana has been chosen". Nothing arranges this deed today; it
        // costs one line to do the right thing if something ever does, and
        // the run itself re-checks the tap against the current
        // `LegalActions` exactly as the two above do.
        Deed::Run {
            plan,
            then: crate::RunEnd::Float,
        } => {
            duel.last_error = None;
            duel.mana_run = Some(crate::ManaRun::new(
                plan,
                armed.object,
                crate::RunEnd::Float,
            ));
        }
    }
}

/// What `object` is offering, if anything.
/// English deliberately: only the `action` on each option is read here —
/// what is drawn is [`crate::hud`]'s business, and this path picks an ability
/// by position or takes the only one there is.
fn abilities_of(duel: &Duel, object: ObjectId) -> Option<Vec<crate::abilities::AbilityOption>> {
    let view = duel.view.as_ref()?;
    let interaction = duel.interaction.as_ref()?;
    Some(crate::abilities::options_for(
        baylee_client_core::Lang::En,
        view,
        interaction,
        object,
        // Only about the permanent whose sheet is standing. `activate_card`
        // asks this of whatever was clicked, and a sub-bubble opened over a
        // stranger would be the last sheet's question drawn on a new card.
        (duel.ability_menu == Some(object))
            .then(|| duel.asking_tap())
            .flatten(),
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
    mut had_keyboard: Local<bool>,
) {
    let fired = Fired::of(&keys, prefs.keymap());
    // The keystroke that opened the panel is not a keystroke for the box.
    // `G` opens the sheet on a frame where nothing here reads the message
    // queue, so the character is still standing in it when the box takes the
    // keyboard a moment later — and `browser_keys` would type it the first
    // time it looks. Advancing this reader past whatever is pending on the
    // frame the box *gains* the keyboard is the whole guard, and it is the
    // right shape for the click path too: a key pressed before the box was
    // clicked belongs to the table it was pressed over.
    //
    // A bool rather than `typing_epoch`, which counts re-seedings of the
    // platform's own field as well as focus and would drop a character every
    // time `Esc` emptied the box.
    let typing = duel.browser.is_typing();
    if typing && !*had_keyboard {
        typed.clear();
    }
    *had_keyboard = typing;
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
    // And the same again for the browser's filter box, which holds the
    // keyboard from the moment the panel opens until the player hands it back
    // — see `browser_takes_the_keyboard`. Only while it holds it, though: the
    // sheet can stand open for a whole turn, and a released box that went on
    // swallowing every keystroke would be the end of playing with the
    // graveyard visible.
    if browser_keys(fired, &keys, &mut typed, &mut duel) {
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
    // And the cast chooser for the same reason, after it: the two never stand
    // together, so the order between them is arbitrary and the rule is not.
    if cast_menu_keys(fired, &mut duel) {
        return;
    }
    if arrange_keys(fired, &mut duel) {
        return;
    }
    if move_the_cursor(fired, &mut duel) {
        return;
    }
    if aim_and_declare(fired, &mut duel) {
        return;
    }
    // Ahead of the primary key and of the straight answers, which is the
    // whole of its precedence: while the dialog is the surface holding the
    // question it owns both of §6's keys, and `the_click` would otherwise
    // spend Enter on whatever card the pointer happens to be resting on
    // behind the sheet.
    if browser_answer_keys(fired, &mut duel) {
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
        duel.browser.toggle_by_hand();
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

/// The filter box takes the keyboard as the zone dialog opens.
///
/// Every application with a search field does this, and §6's reading of the
/// dialog is the same one — *„ein Ort, an dem man arbeitet"*. It was the
/// other way round until the owner paid for it: the box took the keyboard
/// only on a click in it, so a search term typed into a panel that had just
/// been opened was fifteen bound letters fired at the table instead. One of
/// them (`T`) latched the text view on and *persisted* it, which is how every
/// card in a duel came to be drawn as its own rules text; `K`/`B` and `Y`/`N`
/// reach the engine, and there is no undo there.
///
/// The player takes the keyboard back with `Esc` or `Enter` and the sheet
/// goes on standing — that half of the bargain is untouched, and it is what
/// lets a graveyard stay open through a turn.
///
/// Two things it will not do. **Not on a platform that owns the typing**,
/// because there `is_typing` is what raises a phone's keyboard
/// ([`browser_softkeys`]) and a tap on a pile would put it over the graveyard
/// the player meant to read. And **not for an ordering**, which draws no
/// filter box at all (`hud::tray`: the row says what to do instead) — a
/// keyboard handed to a field that is not on screen is a keyboard nobody can
/// get back.
pub fn browser_takes_the_keyboard(mut duel: ResMut<Duel>, mut was_open: Local<bool>) {
    let open = duel.browser.is_open();
    let opening = open && !*was_open;
    *was_open = open;
    if !opening || crate::softkeys::SoftKeyboard::owns_typing() {
        return;
    }
    if duel
        .interaction
        .as_ref()
        .is_some_and(Interaction::is_ordering)
    {
        return;
    }
    duel.browser.start_typing();
}

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
            // Not a keystroke: autofill and paste arrive as a whole value,
            // and with the caret and selection the element is holding — the
            // `<input>` owns both, and the box draws both now.
            crate::softkeys::SoftKey::Text {
                value,
                cursor,
                anchor,
            } => duel.browser.set_filter_state(&value, cursor, anchor),
            // A caret that moved inside unchanged text is exactly what the
            // arrow keys do on a page, and it used to change nothing here
            // because there was no caret to move.
            crate::softkeys::SoftKey::Caret { cursor, anchor } => {
                duel.browser.place_filter_caret(cursor, anchor);
            }
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
///
/// What it reads is what `lobby::systems::text_field_keys` reads, chord for
/// chord, because the owner named the lobby's boxes as the thing this one
/// should be: a caret that moves by character, word and line, a selection
/// shift extends, Delete beside Backspace, and select-all. It was a string
/// with characters pushed onto the end of it and popped off again, which is
/// none of those. Tab is the one chord the lobby has and this does not —
/// there is no second field on a zone sheet to move to.
fn browser_keys(
    fired: Fired,
    codes: &ButtonInput<KeyCode>,
    typed: &mut MessageReader<KeyboardInput>,
    duel: &mut Duel,
) -> bool {
    use baylee_client_core::textbuf::{Dir, Step};
    // Two fields and one chord table. While the builder holds a caret in one
    // of its rows, every key below goes there instead of into the box — the
    // two are editors of one string and only one of them may be typed into,
    // which is the rule `filterdialog`'s own header states. Written as a
    // target rather than as a second copy of the table: a chord added to one
    // field and not the other is how the lobby's boxes and this one drifted
    // apart in the first place.
    let target = if duel
        .browser
        .builder()
        .is_some_and(|it| it.typing().is_some())
    {
        Caret::Row
    } else if duel.browser.is_typing() {
        Caret::Box
    } else {
        return false;
    };
    // `Cancel` is read *before* the platform bail below, and that ordering is
    // the whole of it: where the browser does the typing every raw key
    // belongs to its `<input>`, so returning first would leave Escape doing
    // nothing at all on a page — not emptying the box, not letting go of it,
    // with only the soft keyboard's own action key as a way out. `fired`
    // carries actions rather than raw keys, so reading it here cannot type a
    // character the `<input>` has already taken.
    if fired.has(Action::Cancel) {
        match target {
            // In a row, Escape gives the row back and leaves the builder
            // open: the way out of the *builder* is its own gear, and a key
            // that shut both would make a mistyped letter cost the panel.
            Caret::Row => duel.browser.in_builder(FilterPanel::stop_typing),
            Caret::Box if duel.browser.filter().is_empty() => duel.browser.stop_typing(),
            Caret::Box => duel.browser.clear_filter(),
        }
        return true;
    }
    // The box still owns the keyboard where the platform does the typing —
    // `browser_softkeys` has already read the value — but the client must not
    // read the raw keys as well, or every character is entered twice.
    if crate::softkeys::SoftKeyboard::owns_typing() {
        return true;
    }
    // The three modifiers a text field reads, once for the whole batch
    // because a key event carries no modifier state of its own. Which one
    // means what is the platform's convention rather than a preference, and
    // it is the lobby's: shift extends a selection, and the reaches past a
    // single character are ⌥/Ctrl for a word and ⌘/Home-End for the line.
    let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let word = codes.any_pressed([
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    let line = codes.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    // ⌘A and Ctrl+A, answered before the text arm so the "a" stays out of the
    // box.
    let command = line || codes.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let reach = if line {
        Step::Line
    } else if word {
        Step::Word
    } else {
        Step::Char
    };
    for event in typed.read() {
        if !event.state.is_pressed() {
            continue;
        }
        let gesture = match &event.logical_key {
            Key::Backspace => Gesture::Back,
            Key::Delete => Gesture::Forward,
            Key::ArrowLeft => Gesture::Move(reach, Dir::Left, shift),
            Key::ArrowRight => Gesture::Move(reach, Dir::Right, shift),
            Key::Home => Gesture::Move(Step::Line, Dir::Left, shift),
            Key::End => Gesture::Move(Step::Line, Dir::Right, shift),
            // "Done" rather than "submit": the rows are already narrowed, so
            // the only thing left to do is hand the keyboard back.
            Key::Enter => Gesture::Done,
            Key::Character(s) if command => {
                if s.eq_ignore_ascii_case("a") {
                    Gesture::SelectAll
                } else {
                    continue;
                }
            }
            Key::Character(s) => Gesture::Insert(s.to_string()),
            Key::Space => Gesture::Insert(" ".to_string()),
            _ => continue,
        };
        gesture.done(target, duel);
    }
    true
}

/// Which of the two fields the keyboard is reaching.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Caret {
    /// The search box itself.
    Box,
    /// A row of the filter builder.
    Row,
}

/// One editing gesture, before it is aimed at a field.
///
/// The chords are read once and the field is chosen once, which is the point:
/// the box and a builder row take the same keys because they are the same
/// kind of thing, and a table written twice is a table that agrees until
/// somebody adds a chord to one half of it.
enum Gesture {
    /// Backspace.
    Back,
    /// Delete.
    Forward,
    /// An arrow, Home or End.
    Move(
        baylee_client_core::textbuf::Step,
        baylee_client_core::textbuf::Dir,
        bool,
    ),
    /// ⌘A.
    SelectAll,
    /// Enter: hand the keyboard back.
    Done,
    /// A character, or a whole run of them from an IME or a paste.
    Insert(String),
}

impl Gesture {
    /// Does it, to whichever field holds the caret.
    fn done(self, target: Caret, duel: &mut Duel) {
        match (target, self) {
            (Caret::Box, Self::Back) => {
                duel.browser.pop_filter();
            }
            (Caret::Box, Self::Forward) => duel.browser.delete_forward(),
            (Caret::Box, Self::Move(step, dir, select)) => {
                duel.browser.move_filter_caret(step, dir, select);
            }
            (Caret::Box, Self::SelectAll) => duel.browser.select_all_filter(),
            (Caret::Box, Self::Done) => duel.browser.stop_typing(),
            (Caret::Box, Self::Insert(text)) => duel.browser.type_text(&text),
            (Caret::Row, Self::Back) => duel.browser.in_builder(|it| {
                it.pop_typed();
            }),
            (Caret::Row, Self::Forward) => {
                duel.browser.in_builder(FilterPanel::delete_typed_forward);
            }
            (Caret::Row, Self::Move(step, dir, select)) => duel
                .browser
                .in_builder(|it| it.move_typing_caret(step, dir, select)),
            (Caret::Row, Self::SelectAll) => duel.browser.in_builder(FilterPanel::select_all_typed),
            (Caret::Row, Self::Done) => duel.browser.in_builder(FilterPanel::stop_typing),
            (Caret::Row, Self::Insert(text)) => duel.browser.type_into_builder(&text),
        }
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
///
/// The doc block was above `browser_softkeys` for as long as it existed —
/// spliced onto the next item's own, so `cargo doc` printed it over the soft
/// keyboard's. Same family as the `still_gliding` splice: the anchor is the
/// closing brace before a block, never the `///` after it.
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
            // Neither the language nor the face names are plumbed here, for
            // the one reason: this reads the *shape* of a creature-type list
            // and never its labels. The renderer is where a row is written.
            crate::choices::options(
                &p,
                baylee_client_core::Lang::En,
                duel.statics.as_ref(),
                &duel.subtype_filter,
                crate::choices::FaceNames::default(),
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

/// Takes back what is armed, and the chosen way with it.
///
/// The two are one decision: [`take_cast_row`] answers this client's question
/// and arms the run in the same press, so an `Esc` that left the answer
/// behind would take back the taps and keep the choice. It is not merely
/// untidy — the answer is spent by the *engine's* `ChooseCastMode`, and that
/// question is still reachable by another route (the free alternative the
/// engine offers with an empty pool needs no run at all), so a forgotten one
/// would be applied to a cast the player made some other way.
pub fn disarm(duel: &mut Duel) {
    let Some(armed) = duel.armed.take() else {
        return;
    };
    if duel
        .cast_answer
        .is_some_and(|(card, _)| card == armed.object)
    {
        duel.cast_answer = None;
    }
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
        disarm(duel);
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
/// The cursor is two-dimensional here and on no other sheet, because this one
/// has two directions in it: a centred **row** of mana pips above a column of
/// written rows. Up and down walk everything there is, as they always have;
/// left and right are the pip strip and jump to it from wherever the cursor
/// stands. [`abilitysheet::step_down`] and [`abilitysheet::step_along`] are
/// the arithmetic and carry the reasons.
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
    let split = crate::abilities::Split::of(&options);
    // The list can shrink under a page that was valid when it was turned to.
    duel.ability_page = abilitysheet::clamp(split.numbered().1, duel.ability_page);
    if fired.has(Action::Cancel) {
        // One step back, not all the way out. A sub-bubble was opened by a
        // press on a row of a sheet that is still the answer to the click,
        // and there is no undo in this client anywhere else either — `Esc`
        // takes back the last thing a player said, which here is "that tap".
        if duel.asking_tap().is_some() {
            duel.ability_tap = None;
            duel.ability_pick = 0;
            duel.ability_page = 0;
        } else {
            duel.ability_menu = None;
        }
        return true;
    }
    let down = i32::from(fired.has(Action::CursorDown)) - i32::from(fired.has(Action::CursorUp));
    let along =
        i32::from(fired.has(Action::CursorRight)) - i32::from(fired.has(Action::CursorLeft));
    if down != 0 || along != 0 {
        // A frame carrying both is answered by the strip, because that is the
        // gesture that names a destination rather than a direction.
        duel.ability_pick = if along == 0 {
            abilitysheet::step_down(options.len(), duel.ability_pick, down)
        } else {
            abilitysheet::step_along(options.len(), split.pips, duel.ability_pick, along)
        };
        // The cursor walks the whole list and the sheet shows nine rows of
        // it, so walking off the end of a page turns it. Deriving the page
        // from the cursor rather than moving them separately is what stops
        // the highlight from being on a row that is not drawn — and the pips
        // are drawn on every page, so a cursor on one leaves the page where
        // it was rather than sending it back to the first.
        duel.ability_page = if duel.ability_pick < split.pips && split.rows > 0 {
            duel.ability_page
        } else {
            split.page_of(duel.ability_pick)
        };
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
    // The digits count the **written** rows and skip the pips a mana header
    // leads with, which is [`crate::abilities::Split::numbered`]: a pip on a
    // sheet carries no keycap, so a digit that reached one would send a row
    // nothing on screen had numbered.
    let (head, counted) = crate::abilities::Split::of(&options).numbered();
    let page = abilitysheet::clamp(counted, duel.ability_page);
    duel.ability_page = page;
    if digit == abilitysheet::PAGER {
        // The pager is drawn only where there is a second page, so a `0` on a
        // sheet of two rows is a key nothing on screen offered: it consumes
        // no frame and turns nothing.
        turn_the_page(duel);
        return abilitysheet::paged(counted);
    }
    let Some(at) = abilitysheet::option_of(counted, page, digit) else {
        return false;
    };
    take_sheet_row(duel, head + at);
    true
}

/// The row a press landed on: armed by the first press, sent by the second.
///
/// The digit and the click share it, because they are the same press — the
/// keycap is what the digit is drawn on, and a row whose click armed while
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
    let (head, counted) = crate::abilities::Split::of(&options).numbered();
    if !abilitysheet::paged(counted) {
        return;
    }
    let page = abilitysheet::clamp(counted, duel.ability_page);
    duel.ability_page = abilitysheet::turn(counted, page);
    duel.ability_pick = head + abilitysheet::rows(counted, duel.ability_page).start;
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
    if fired.has(Action::ActivateGroup)
        && let Some(object) = duel.hovered
    {
        activate(duel, object, true);
        return true;
    }
    false
}

/// While an arrangement holds a card, the cursor keys move it — one place
/// along its pile, or to the end of the pile above or below — and reach
/// nothing else.
///
/// A held card is the one moment those keys have a better meaning than
/// walking the table behind the sheet: the focus keys already walk the
/// cards, and the tick already takes one up and puts it down in front of
/// another, so what a keyboard was missing is the small move a pointer
/// makes by tapping the neighbour. With nothing held they are the table's
/// again. Returns whether it consumed the frame.
fn arrange_keys(fired: Fired, duel: &mut Duel) -> bool {
    if !duel.browser.answers_here(duel.interaction.as_ref()) {
        return false;
    }
    let Some(i) = duel.interaction.as_mut() else {
        return false;
    };
    if i.arrangement().and_then(Arrangement::held).is_none() {
        return false;
    }
    let mut consumed = false;
    for (action, nudge) in [
        (Action::CursorLeft, Nudge::Earlier),
        (Action::CursorRight, Nudge::Later),
        (Action::CursorUp, Nudge::PrevRow),
        (Action::CursorDown, Nudge::NextRow),
    ] {
        if fired.has(action) {
            i.nudge(nudge);
            consumed = true;
        }
    }
    consumed
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

/// The dialog's own keyboard, for the two keys that meant something else.
///
/// `docs/redesign-proposal.md` §6 is the spec: "Tab moves through rows, Space
/// toggles, Enter confirms". [`Action::CombatFocusNext`] was already the first
/// of those — its own doc says walking a `Mode::Objects` offer is the same
/// gesture as aiming at a blocker — and the other two both belonged to
/// something else.
///
/// [`Action::Confirm`] means "I am done here" *and* "pass priority", which is
/// right in every window but two, and this is the first of them: a
/// `ChooseCards { min: 0 }` arrives while a player is passing priority through
/// a stack of triggers, and the next press in that rhythm answered it with
/// "nothing found" — silently, with no trace and no undo on the wire. A Solemn
/// Simulacrum's search for a basic land was thrown away that way twice, and
/// the land count never moved. The second window is the combat declaration,
/// which is not a sheet and so is guarded where the key is read instead; see
/// [`committed_answer`].
///
/// [`Action::Primary`] is Enter, and it did reach confirm — but only as
/// [`the_click`]'s *third* branch, behind the card under the pointer. A card
/// under the pointer is the ordinary state of a table with a dialog standing
/// over it, so the key §6 gives the dialog was the one key the dialog was
/// least likely to get: the pointer resting on the permanent whose ability
/// asked the question is enough to take it.
///
/// So while the dialog is the surface holding the question, both keys belong
/// to it. The confirm key ticks the focused row instead of sending — a stray
/// press then does something the player can see and take back — and the
/// primary key sends. Neither reaches the table behind the sheet, which is
/// the whole point: `answers_here` is only true when the answer is *not* on
/// the table, because [`baylee_client_core::browser::Browser::follow`] opens
/// the sheet for a choice exactly when nothing on the table can answer it.
///
/// Returns whether it consumed the frame.
fn browser_answer_keys(fired: Fired, duel: &mut Duel) -> bool {
    if !duel.browser.answers_here(duel.interaction.as_ref()) {
        return false;
    }
    if fired.has(Action::Primary) {
        // Consumes the frame even when the answer is not complete — a `min`
        // not reached yet, so `confirm` says no. The alternative is Enter
        // falling through to the hovered card, which is the defect this
        // branch exists to close, and it would fire on exactly the presses a
        // player makes while still building the answer.
        if let Some(action) = committed_answer(duel) {
            duel.submit(action);
        }
        return true;
    }
    if !fired.has(Action::Confirm) {
        return false;
    }
    // Consumes the frame even when the focus stands on nothing selectable:
    // the point is that this key does not reach `confirm` while the dialog
    // is up, and a `Rejected` that fell through would reach it.
    if let Some(i) = duel.interaction.as_mut() {
        i.toggle_focused();
    }
    true
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
    if let Some(action) = committed_answer(duel) {
        duel.submit(action);
        return true;
    }
    false
}

/// Whether the question standing is a combat declaration with nothing
/// declared **and** something that could still be declared.
///
/// The second half is what keeps this from being a nuisance. A
/// `ChooseAttackers` whose `attackers` list is empty is a question with one
/// possible answer, and refusing the confirm key there would stop a player
/// walking a turn forward at a combat step that was never going to hold
/// anything. What the guard is for is the case where an attack exists and is
/// about to be thrown away.
///
/// Read off [`Interaction::pending`] and [`Interaction::declared`], both of
/// which this crate already had: the predicate is a client policy about which
/// *key* may send an answer, not a claim about which answers are legal, so it
/// deliberately does not live next to [`Interaction::can_confirm`] — which is
/// right as it stands, because declaring nothing **is** legal.
#[must_use]
pub(crate) fn empty_combat_declaration(interaction: &Interaction) -> bool {
    use baylee_engine::choice::Pending;
    if interaction.declared() > 0 {
        return false;
    }
    match interaction.pending() {
        Pending::ChooseAttackers { attackers, .. } => !attackers.is_empty(),
        Pending::ChooseBlockers { blockers, .. } => !blockers.is_empty(),
        _ => false,
    }
}

/// The answer a *confirm* key or button may send, which is not quite
/// [`Interaction::confirm`].
///
/// One exception, and it is the same one [`browser_answer_keys`] closed a
/// window over. [`Action::Confirm`] means "I am done here" *and* "pass
/// priority", so a player walking a turn forward presses it in a rhythm — and
/// a combat declaration arrives inside that rhythm. The next press in it
/// answered the question with `DeclareAttackers { attackers: [] }`: the attack
/// step spent, a 2/1 left untapped against an open opponent, nothing said on
/// the prompt bar and nothing on the wire to take back. It was found by
/// playing, not by reading, which is why it survived a `ChooseCards { min: 0 }`
/// being fixed for the same reason one window over.
///
/// So an **empty** combat declaration no longer reaches the engine through a
/// confirm key. It is still a real answer, and `O` — [`Action::CombatNone`],
/// [`PromptAction::DeclareNothing`] — is what sends it, through
/// [`declare_nothing`], which calls [`Interaction::confirm`] directly and is
/// deliberately *not* routed through here. Declining therefore costs a key of
/// its own, which is the whole repair: the press that declines is no longer
/// the press that was already being made.
///
/// Nothing is written to [`Duel::last_error`] on a refused press. Every
/// client-side refusal there is an English constant in an otherwise translated
/// interface, and adding a fourth is work this client owes once, not five
/// times. The feedback this refusal needs is drawn instead:
/// `hud::ledge::answers_for` takes the confirm answer off the prompt bar for
/// exactly the state this function refuses, so the key is unadvertised at the
/// moment it stops working, with the two answers that do something beside it.
fn committed_answer(duel: &Duel) -> Option<PlayerAction> {
    let interaction = duel.interaction.as_ref()?;
    if empty_combat_declaration(interaction) {
        return None;
    }
    interaction.confirm()
}

/// Every straight answer to a pending choice.
///
/// Each goes through the interaction, which refuses it unless the engine
/// actually asked — so a key bound to "yes" does nothing at all during
/// combat, without this function knowing what combat is.
fn answer_the_question(fired: Fired, duel: &mut Duel, prefs: &mut crate::prefs::Prefs) {
    // Confirm / pass priority. Never toggles anything else, so it is the one
    // key that always means "I am done here" — with the one exception
    // [`committed_answer`] names.
    if fired.has(Action::Confirm)
        && let Some(action) = committed_answer(duel)
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

    // Cancel: an open preview first, then the game menu, then the zone
    // browser, then a selected phase button, then a half-built answer.
    //
    // The browser sits where it does because Esc walks the screen from the
    // top down and the sheet is a *standing* panel: the preview is over it
    // and is gone the moment the pointer moves, while the sheet stays until
    // it is put away. Its filter box comes earlier still, in `browser_keys` —
    // a box that has the keyboard answers Escape itself.
    //
    // It also has to be *puttable* away, and a sheet a question opened is
    // not: this branch used to close one, `Browser::follow` re-opened it on
    // the next frame, and Escape looked like a key nothing had wired. Now the
    // branch is not taken and Escape falls through to the answer the question
    // is holding — clearing a half-built selection, which is the thing a
    // player pressing Escape in front of a question actually means.
    if fired.has(Action::Cancel) {
        if duel.hovered.is_some() {
            duel.hovered = None;
            duel.hovered_at = None;
        } else if duel.game_menu {
            // Above the browser and below the preview, because `Esc` walks
            // the screen from the top down and this panel stands over the
            // strip the browser is put away into. It is also the newer of the
            // two standing panels in every case where both are up: the
            // browser can stand open for a whole turn, and nobody opens the
            // menu and then forgets it.
            duel.game_menu = false;
        } else if duel.browser.is_open() && duel.browser.may_be_put_away() {
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
    closes: Query<&TrayMinimise>,
    grows: Query<&crate::hud::TrayMaximise>,
    tabs: Query<&TrayTab>,
    parents: Query<&ChildOf>,
    windows: Query<&Window>,
    mut panels: Query<&mut Node, With<crate::hud::TrayPanel>>,
    mut duel: ResMut<Duel>,
    mut settings: ResMut<ClientSettings>,
    mut revision: ResMut<crate::hud::TrayRevision>,
) {
    use crate::hud::{TrayDrag, TrayDragKind};

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
        // The minimise button and the zone tabs sit *on* the header, so
        // their lineage carries the grip. The specific control claims the
        // press before the row it stands on does, or putting the sheet away
        // would first nudge it by whatever the hand wobbled between the press
        // and the release — and then save that.
        //
        // The tabs joined that list when they moved into the title row on
        // 14.09.2026. It is the bargain the minimise button already had, and
        // it is the reason the tabs could move at all: a chip that started a
        // drag would carry the whole sheet sideways every time a pile was
        // ticked. The maximise button joined on 19.09.2026 when it moved up
        // out of the corner and into the row beside its neighbour — which is
        // the third time this list has had to grow with the header, and the
        // reason it is one condition over three queries rather than a rule
        // about where a control happens to sit.
        if find_in_lineage(down.entity, &closes, &parents).is_some()
            || find_in_lineage(down.entity, &grows, &parents).is_some()
            || find_in_lineage(down.entity, &tabs, &parents).is_some()
        {
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
    // Whether a drag *ended* this frame, which is not the same as whether the
    // button came up: a release with no drag under it is a click somewhere
    // else on the sheet and has nothing to write. The drag itself is no
    // longer wanted — it was read for how far it had travelled, back when the
    // corner had a second job.
    let mut ended = None;
    for _up in ups.read() {
        ended = ended.or_else(|| duel.tray_drag.take());
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

    // The corner used to maximise as well, on a press and release that
    // travelled less than a `TAP_SLOP` of 4 px between them. It was found
    // this way rather than through a `Pointer<Click>` because a resize *ends*
    // over the corner — the corner travels under the hand — so every drag
    // would have fired one; and it existed at all because the corner drew a
    // ⤢, which is an argument from a mark rather than from a control. The
    // mark and the gesture both moved into the head on 19.09.2026
    // (`hud::TrayMaximise`), where a click is a click and nothing has to be
    // inferred from how far a hand wandered. The corner resizes.
    if let Some(drag) = ended {
        settings.save();
        // A *resize* ends with the sheet a different size than the one it was
        // built at, so the sheet is rebuilt once — the grid's tiles are
        // packed from the width and a stretched sheet keeps the packing it
        // had. A move is not asked for: nothing about the sheet's contents
        // depends on where in the band it stands.
        if drag.kind == TrayDragKind::Resize {
            revision.relayout();
        }
    }
}

/// Flies the sheet between two rectangles, and stops.
///
/// The counterpart of `hud::reveal_tray`: that one carries the sheet in and
/// out of the tray, this one carries it between two sizes. They are two
/// systems because they move two different things — a `UiTransform` there, a
/// `Node` here — and [`TrayGlide`]'s own docs carry why this one cannot be a
/// transform.
///
/// It is in `input` rather than in `hud` for one reason and it is a good one:
/// [`write_placement`] is here, because a drag is the other thing that moves
/// the sheet without going through the revision, and two writers of one
/// `Node` in two modules is how they come to disagree about which of them
/// owns it. A drag and a glide never run together — `tray_drag` takes the
/// press, and the maximise button is excluded from it.
///
/// `reduce_motion` is honoured the way everything else here honours it: the
/// movement still happens, it is simply already over on the frame it started.
pub fn glide_the_sheet(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut glide: ResMut<TrayGlide>,
    mut revision: ResMut<crate::hud::TrayRevision>,
    mut panels: Query<&mut Node, With<crate::hud::TrayPanel>>,
) {
    /// How long the sheet takes to change size.
    ///
    /// The ability sheet's own span (`hud::motion::ZOOM_IN`), because this is
    /// the same claim about the same interface: §7 measures everything
    /// against "160 ms ease-out-back", and a panel resizing is no more
    /// important than a panel arriving.
    const GLIDE: f32 = 0.16;

    let Some(flight) = glide.0.as_mut() else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    flight.t = if still {
        1.0
    } else {
        (flight.t + time.delta_secs() / GLIDE).min(1.0)
    };
    // Ease-out, with no overshoot at all. A rectangle that overshot would put
    // an edge of the sheet outside the band for two frames — `lerp` does not
    // clamp, on purpose — and the one place this movement ends is exactly the
    // rectangle the store already holds.
    let eased = 1.0 - (1.0 - flight.t).powi(3);
    let at = flight.from.lerp(flight.to, eased);
    write_placement(&mut panels, at);
    if flight.t >= 1.0 {
        glide.0 = None;
        // And one rebuild, now that the sheet has stopped. The head's
        // maximise button is drawn from `is_maximised` and the grid's tiles
        // are packed from the sheet's width, and both were decided the last
        // time `sync_tray` ran — which was before any of this moved. Without
        // it a maximised sheet still offers to maximise.
        revision.relayout();
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
///
/// `pub` for the same reason [`activate_card`] is: a row of this chooser is a
/// press, and a test that built the answer by hand would pass just as loudly
/// with no button behind it.
pub fn pick_choice(duel: &mut Duel, index: usize) {
    // This client's own chooser first, because while it stands it *is* the
    // question on the bar — the engine is still holding an ordinary priority
    // window behind it. See [`take_cast_row`].
    if duel.cast_menu.is_some() {
        take_cast_row(duel, index);
        return;
    }
    let offered = duel
        .interaction
        .as_ref()
        .map(baylee_client_core::Interaction::prompt)
        // The language and the face names are irrelevant here and
        // deliberately not plumbed: only the *shape* of the answer is read
        // back -- whether this prompt is an indexed choice at all, and how
        // many rows it has. The labels are the renderer's business.
        .and_then(|p| {
            crate::choices::options(
                &p,
                baylee_client_core::Lang::En,
                duel.statics.as_ref(),
                &duel.subtype_filter,
                crate::choices::FaceNames::default(),
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

/// Takes one row of the cast chooser: the way is remembered, the deed is
/// armed, the chooser closes.
///
/// It **arms** rather than sending, which is the same two-stage rule the
/// ability sheet's rows follow ([`take_sheet_row`]) and for the same reason:
/// there is no undo in the engine, and a spell on the stack is the least
/// undoable thing in the game. So the press that answers this client's
/// question leaves the deed standing in the prompt bar, and the next one
/// sends it.
///
/// The list is rebuilt from the current `LegalActions` before the row is
/// read, exactly as every other chooser in this file does — a bar drawn a
/// frame ago must not be able to pick a way the board no longer offers.
fn take_cast_row(duel: &mut Duel, at: usize) {
    let Some(card) = duel.cast_menu.as_ref().map(|m| m.card) else {
        return;
    };
    let Some(mode) = cast_menu_for(duel, card).and_then(|m| m.mode(at).cloned()) else {
        // The card has stopped offering that many ways. Close rather than
        // guess: the player is looking at a list that is no longer true.
        duel.cast_menu = None;
        duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn));
        return;
    };
    duel.cast_menu = None;
    duel.cast_answer = Some((card, mode.kind));
    arm(
        duel,
        card,
        Deed::Run {
            plan: mode.plan,
            then: crate::RunEnd::Cast,
        },
    );
}

/// The open cast chooser: the cursor keys walk it, the primary key or confirm
/// takes the row, cancel puts it away. Returns whether it consumed the frame.
///
/// Its own handler rather than a branch of [`ability_menu_keys`], because the
/// two menus are about different things and never stand together — but the
/// same shape, so the keyboard answers this list the way it answers that one.
/// The rows are a single column here, so up and down are the whole of the
/// walk.
///
/// "Never together" is a property of one place and not an observation:
/// [`activate_card`] is the only door either menu is opened through, and it
/// closes the other one before it takes any branch at all.
pub fn cast_menu_keys(fired: Fired, duel: &mut Duel) -> bool {
    let Some(card) = duel.cast_menu.as_ref().map(|m| m.card) else {
        return false;
    };
    let Some(fresh) = cast_menu_for(duel, card) else {
        // Fewer than two ways left: the chooser is stale, and holding it open
        // would keep the keyboard hostage over a question that has answered
        // itself.
        duel.cast_menu = None;
        return false;
    };
    let len = fresh.modes.len();
    if let Some(menu) = duel.cast_menu.as_mut() {
        menu.modes = fresh.modes;
        menu.pick = menu.pick.min(len - 1);
    }
    if fired.has(Action::Cancel) {
        duel.cast_menu = None;
        return true;
    }
    let step = i32::from(fired.has(Action::CursorDown)) - i32::from(fired.has(Action::CursorUp))
        + i32::from(fired.has(Action::CursorRight))
        - i32::from(fired.has(Action::CursorLeft));
    if step != 0 {
        if let Some(menu) = duel.cast_menu.as_mut() {
            let at = i32::try_from(menu.pick).unwrap_or(0);
            let wide = i32::try_from(len).unwrap_or(1);
            menu.pick = usize::try_from((at + step).rem_euclid(wide)).unwrap_or(0);
        }
        return true;
    }
    if fired.has(Action::Primary) || fired.has(Action::Confirm) || fired.has(Action::ActivateCard) {
        let at = duel.cast_menu.as_ref().map_or(0, |m| m.pick);
        take_cast_row(duel, at);
        return true;
    }
    false
}

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
fn sheet_click(
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
fn browser_click(
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
fn shift_held(keys: Option<&ButtonInput<KeyCode>>) -> bool {
    keys.is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]))
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
            pick_choice(&mut duel, button.index);
            continue;
        }
        if browser_click(&mut duel, e, &mut tray, &parents) {
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
                PromptAction::Confirm => committed_answer(&duel),
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
fn select_stack_stop(duel: &mut Duel, object: ObjectId) -> bool {
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod refusal_tests;

/// The zone browser's sheet actually moves when its header is dragged.
///
/// A drag cannot be proved through the `dev-control` harness — `/pointer`
/// presses and releases in one call, so there is no frame in the middle where
/// the cursor is somewhere else — and "declared but never wired" is a bug this
/// client has shipped before. So it is proved here instead: the system is put
/// in an `App` with a window, a sheet and the two markers, and what is
/// asserted is the sheet's own `Node`, not that `update()` returned.
#[cfg(test)]
mod dragging;
