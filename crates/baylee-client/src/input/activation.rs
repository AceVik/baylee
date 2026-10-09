//! Activating a card: what a press on it means, arming, firing an armed deed.

#[allow(clippy::wildcard_imports)] // the input module's shared vocabulary
use super::*;

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
pub(super) fn card_on_screen(
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
    // Reachable before castable: a card is both only when the engine offers
    // it for Phyrexian life alone and lands could pay the mana instead
    // (`reachable` in lib.rs), and then the lands are tapped first so the
    // engine asks how to pay rather than taking the life.
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
    if let Some(action) = duel.interaction.as_ref().and_then(|i| i.play_card(object)) {
        // A land plays on the click (`Interaction::plays_only_as_a_land`
        // draws the line: a modal card with a spell side opens a chooser).
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
    // Mana and tap-only actions retain their one-click path. A costly
    // ability opens its sentence while armed, even when it is the only row.
    if let Some(options) = abilities_of(duel, object) {
        match options.len() {
            0 => {}
            1 => {
                duel.ability_menu = None;
                if !arm_ability(duel, object, &options[0]) && !options[0].mana {
                    open_ability_sheet(duel, object);
                }
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
    let card = baylee_client_core::decision::known_cards(view).find(|card| card.id == object)?;
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
pub(super) fn cast_menu_for(duel: &Duel, object: ObjectId) -> Option<crate::CastMenu> {
    let view = duel.view.as_ref()?;
    let legal = duel.interaction.as_ref()?.legal_actions()?;
    if !legal.castable.contains(&object)
        && !legal.lands.contains(&object)
        && !duel.reachable.contains(&object)
    {
        return None;
    }
    let modes = crate::castmodes::reachable_modes(view, legal, object);
    (modes.len() > 1
        || modes.first().is_some_and(|m| {
            matches!(
                m.kind,
                baylee_engine::choice::CastModeKind::Kicked
                    | baylee_engine::choice::CastModeKind::Prototype
                    | baylee_engine::choice::CastModeKind::Disguise
            )
        }))
    .then_some(crate::CastMenu {
        card: object,
        modes,
        pick: 0,
    })
}

/// Arms a deed. The prompt bar draws what is armed, and the way out of it.
pub(super) fn arm(duel: &mut Duel, object: ObjectId, deed: Deed) {
    duel.armed = Some(crate::Armed { object, deed });
}

/// Re-read the chosen spell mode after any manual mana taps. A land face in
/// the same card must never stand in for the spell the player selected.
pub(crate) fn chosen_cast_plan(
    duel: &Duel,
    object: ObjectId,
) -> Option<baylee_client_core::manaplan::Plan> {
    let (card, kind) = duel.cast_answer?;
    if card != object {
        return None;
    }
    crate::castmodes::reachable_modes(
        duel.view.as_ref()?,
        duel.interaction.as_ref()?.legal_actions()?,
        object,
    )
    .into_iter()
    .find(|mode| mode.kind == kind)
    .map(|mode| mode.plan)
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
pub(super) fn arm_ability(
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
    if let PlayerAction::ActivateAbility { ability_index, .. } = option.action
        && let (Some(view), Some(legal)) = (
            duel.view.as_ref(),
            duel.interaction
                .as_ref()
                .and_then(Interaction::legal_actions),
        )
        && let Some(plan) = crate::abilities::mana_for(view, legal, object, ability_index)
    {
        arm(
            duel,
            object,
            Deed::Run {
                plan,
                then: crate::RunEnd::Ability(ability_index),
            },
        );
        return false;
    }
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

/// Confirm the chosen spell independently of any land play the same card offers.
fn fire_cast_payment(duel: &mut Duel, object: ObjectId, plan: baylee_client_core::manaplan::Plan) {
    // The short-circuit below is for the *manual* land tap that may
    // have happened between the two clicks, and it must not fire when
    // the player has chosen a way to cast: the engine's answer with
    // the pool as it stands is exactly the wrong one — a Solitude
    // whose printed cost the player picked is `castable` this whole
    // time, for its free evoke. Re-plan the chosen mode, then use
    // the run even with zero taps: its final action is explicitly a
    // cast, while `play_card` prefers a legal MDFC land face.
    let chosen = duel
        .cast_answer
        .as_ref()
        .is_some_and(|(card, _)| *card == object);
    // A card the pool cannot pay for yet is cast first and paid for in the
    // window its cast opens (CR 601.2g), so the mana's triggers wait until
    // it is cast (CR 601.2i). The plan that made it reachable is only the
    // promise that the window can be paid.
    let payable = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .is_some_and(|legal| legal.payable.contains(&object));
    if chosen {
        let Some(plan) = chosen_cast_plan(duel, object) else {
            duel.cast_answer = None;
            duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn));
            return;
        };
        duel.last_error = None;
        duel.mana_run = Some(if payable {
            crate::ManaRun::cast_first(object)
        } else {
            crate::ManaRun::new(plan, object, crate::RunEnd::Cast)
        });
    } else if let Some(action) = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .filter(|legal| legal.castable.contains(&object))
        .map(|_| PlayerAction::CastSpell { card: object })
    {
        duel.submit(action);
    } else if duel.reachable.contains(&object) {
        duel.last_error = None;
        duel.mana_run = Some(if payable {
            crate::ManaRun::cast_first(object)
        } else {
            crate::ManaRun::new(plan, object, crate::RunEnd::Cast)
        });
    } else {
        duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn));
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
            then: crate::RunEnd::Ability(index),
            ..
        } => fire_ability_payment(duel, armed.object, index),
        Deed::Run {
            plan,
            then: crate::RunEnd::Cast,
        } => fire_cast_payment(duel, armed.object, plan),
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
        // A settle is never armed either (`Duel::pay_owed` starts it on the
        // press); should anything arm one, it runs the same way.
        Deed::Run {
            plan,
            then: then @ (crate::RunEnd::Float | crate::RunEnd::Settle),
        } => {
            duel.last_error = None;
            duel.mana_run = Some(crate::ManaRun::new(plan, armed.object, then));
        }
    }
}

fn open_ability_sheet(duel: &mut Duel, object: ObjectId) {
    duel.ability_menu = Some(object);
    duel.ability_pick = 0;
    duel.ability_page = 0;
    duel.ability_tap = None;
}

fn fire_ability_payment(duel: &mut Duel, object: ObjectId, index: u32) {
    let offered = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .is_some_and(|legal| legal.abilities.contains(&(object, index)));
    if offered {
        duel.submit(PlayerAction::ActivateAbility {
            source: object,
            ability_index: index,
        });
    } else if let (Some(view), Some(legal)) = (
        duel.view.as_ref(),
        duel.interaction
            .as_ref()
            .and_then(Interaction::legal_actions),
    ) && let Some(plan) = crate::abilities::mana_for(view, legal, object, index)
    {
        duel.last_error = None;
        duel.mana_run = Some(crate::ManaRun::new(
            plan,
            object,
            crate::RunEnd::Ability(index),
        ));
    } else {
        duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn));
    }
}

/// What `object` is offering, if anything.
/// English deliberately: only the `action` on each option is read here —
/// what is drawn is [`crate::hud`]'s business, and this path picks an ability
/// by position or takes the only one there is.
pub(super) fn abilities_of(
    duel: &Duel,
    object: ObjectId,
) -> Option<Vec<crate::abilities::AbilityOption>> {
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
