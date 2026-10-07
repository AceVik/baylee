//! Answering the question: the click, committed answers, numbers, focus.

#[allow(clippy::wildcard_imports)] // the input module's shared vocabulary
use super::*;

/// The primary key, with its fixed precedence: the card under the cursor,
/// then the selected phase button, then confirm. Returns whether it consumed
/// the frame.
pub(super) fn the_click(fired: Fired, duel: &mut Duel, prefs: &mut crate::prefs::Prefs) -> bool {
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
pub(super) fn committed_answer(duel: &Duel) -> Option<PlayerAction> {
    let interaction = duel.interaction.as_ref()?;
    if empty_combat_declaration(interaction) {
        return None;
    }
    interaction.confirm()
}

/// Why a Confirm sent nothing, where a bound the question states held the
/// answer back (creatures short of a crew's power, one creature blocking an
/// attacker with menace): the reason the engine would have refused it with,
/// said on the table instead of sent.
pub(super) fn say_why_held_back(duel: &mut Duel) {
    if let Some(fault) = duel
        .interaction
        .as_ref()
        .and_then(baylee_client_core::Interaction::answer_fault)
    {
        duel.last_error = Some(baylee_client_core::i18n::Refusal::Verbatim(
            fault.reason().to_string(),
        ));
    }
}

/// Every straight answer to a pending choice.
///
/// Each goes through the interaction, which refuses it unless the engine
/// actually asked — so a key bound to "yes" does nothing at all during
/// combat, without this function knowing what combat is.
pub(super) fn answer_the_question(fired: Fired, duel: &mut Duel, prefs: &mut crate::prefs::Prefs) {
    if fired.has(Action::ConfirmTargetBatch)
        && let Some(answer) = duel
            .interaction
            .as_ref()
            .zip(duel.view.as_ref())
            .and_then(|(i, v)| baylee_client_core::targeting::batch_answer(i, v))
    {
        duel.submit(answer);
        return;
    }

    // Confirm / pass priority. Never toggles anything else, so it is the one
    // key that always means "I am done here" — with the one exception
    // [`committed_answer`] names.
    if fired.has(Action::Confirm) {
        if let Some(action) = committed_answer(duel) {
            duel.submit(action);
            return;
        }
        say_why_held_back(duel);
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
            duel.yes_batch = crate::yes_batch::YesBatch::default();
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

    // Cancel: an open preview first, then cards another seat revealed, then
    // the game menu, then the zone browser, then a selected phase button,
    // then a half-built answer.
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
        } else if duel.reveals.current().is_some() {
            // Under the preview, which goes with the pointer anyway, and over
            // every standing panel: it is the newest thing on the screen and
            // the only one that arrived without the player asking for it.
            // One press puts one reveal away; the next waiting one stands up.
            duel.reveals.dismiss();
        } else if duel.game_menu {
            // Above the browser and below the preview, because `Esc` walks
            // the screen from the top down and this panel stands over the
            // strip the browser is put away into. It is also the newer of the
            // two standing panels in every case where both are up: the
            // browser can stand open for a whole turn, and nobody opens the
            // menu and then forgets it.
            duel.game_menu = false;
        } else if duel.log_open {
            // Under the menu and over the browser. It stands beside the strip
            // the browser is put away into, and like the menu it is only ever
            // up because the player put it there; the browser can have been
            // opened by a question, and that one is answered, not dismissed.
            duel.log_open = false;
        } else if duel.browser.is_open() && duel.browser.may_be_put_away() {
            duel.browser.close();
        } else if prefs.orders().selected().is_some() {
            prefs.rail_cursor().clear_selection();
        } else if duel.visiting.is_some()
            && duel
                .interaction
                .as_ref()
                .is_none_or(|i| i.selected().next().is_none() && i.assignments().is_empty())
        {
            // Lowest of all: a visit is a view, and `Esc` is "back" once
            // nothing on the screen or in the answer is left to take back
            // (DESIGN-v7 §2.4).
            navigate_home(duel);
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
pub(super) fn step_number(duel: &mut Duel, delta: i32) {
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
pub(super) fn cycle_combat_focus(duel: &mut Duel, delta: i32) {
    if let Some(i) = duel.interaction.as_mut() {
        i.cycle_focus(delta);
    }
}

/// Declares nothing and moves on — no attackers, or no blockers.
///
/// Routed through the interaction rather than sent as an empty action
/// directly, so an empty declaration is validated exactly like a full one and
/// the key does nothing at all outside combat.
pub(super) fn declare_nothing(duel: &mut Duel) {
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
